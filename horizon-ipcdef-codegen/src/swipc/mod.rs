//! This implements a parser and a data model for (a variant of) [SwIPC](https://github.com/reswitched/SwIPC) files
//!
//! Notable changes:
//! - Replace most of the built-in types with atmosphere's abstractions
//!   so, buffer -> sf::*Buffer; pid -> sf::ClientProcessId; etc
//! - No more typed buffers, only bytes. Use either LargeData marker for the struct or an array
//!    So, no more `buffer<data_type, transfer_type, size>`
//! - Allow (and prefer) symbolic names for buffer transfer types
//! - do not put the placeholder u64 when sending our own PID using the kernel, it will be done automagically (see [here](https://discord.com/channels/269333940928512010/383368936466546698/994962645906108426))
//! - remote nn:: namespace prefix (we are not nintendo)
//! - use atmosphere's sf::Out markers for outputs, removing the `->` part altogether
//! - ???

use lalrpop_util::lalrpop_mod;

pub mod diagnostics;
pub mod model;

lalrpop_mod!(parser, "/swipc/swipc.rs");

#[cfg(test)]
mod tests {
    use crate::swipc::diagnostics::{diagnostic_error_from_parse_error, Span};
    use crate::swipc::model::{
        BufferTransferMode, IntType, Interface, IpcFile, NominalType, Struct, StructField,
        TypeAlias,
    };
    use crate::swipc::parser;
    use codespan_reporting::diagnostic::Diagnostic;
    use codespan_reporting::term::termcolor::Buffer;
    use lalrpop_util::lexer::Token;
    use std::default::Default;
    use std::fmt::Debug;

    type ParseError<'a> = lalrpop_util::ParseError<usize, Token<'a>, Vec<Diagnostic<usize>>>;

    fn display_error(source: &str, error: ParseError) -> String {
        let (files, diagnostics) = diagnostic_error_from_parse_error(source, error);

        let mut writer = Buffer::ansi(); //StandardStream::stdout(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();

        for diag in diagnostics {
            codespan_reporting::term::emit(&mut writer, &config, &files, &diag).unwrap();
        }

        String::from_utf8(writer.into_inner()).expect("Non utf-8 error output...")
    }

    fn unwrap_parse<T>(source: &str, parser: impl FnOnce(&str) -> Result<T, ParseError>) -> T {
        match parser(source) {
            Ok(r) => r,
            Err(error) => {
                let err = display_error(source, error);
                panic!("{}", err);
            }
        }
    }

    fn unwrap_err_parse<'a, T: Debug>(
        source: &str,
        parser: impl FnOnce(&str) -> Result<T, ParseError>,
        str_match: &str,
    ) {
        match parser(source) {
            Ok(r) => {
                panic!("Parse error should have occurred; parsed:\n{:#?}", r);
            }
            Err(error) => {
                let err = display_error(source, error);
                println!("{}", err);
                assert!(
                    err.contains(str_match),
                    "Could find the expected pattern in the error"
                );
            }
        }
    }

    fn parse_type_alias(s: &str) -> Result<TypeAlias, ParseError> {
        parser::TypeAliasParser::new().parse(0, s)
    }

    #[test]
    fn simple_type_alias() {
        let t: TypeAlias = unwrap_parse("type hello::world = u8;", parse_type_alias);
        assert_eq!(t.name, "hello::world");
        assert_eq!(t.referenced_type, NominalType::Int(IntType::U8));
    }

    #[test]
    fn name_type_alias() {
        let t: TypeAlias = unwrap_parse(r"type some_struct = some_other_struct;", parse_type_alias);
        assert_eq!(t.name, "some_struct");
        assert_eq!(
            t.referenced_type,
            NominalType::TypeName {
                name: arcstr::literal!("some_other_struct"),
                reference_location: Span::default(),
            },
        );
    }

    fn parse_struct_def(s: &str) -> Result<Struct, ParseError> {
        parser::StructDefParser::new().parse(0, s)
    }

    #[test]
    fn struct_def() {
        let t: Struct = unwrap_parse(
            r"
                struct some_struct {
                    u8 bla;
                    sf::Bytes<0x100> buffer;
                };",
            parse_struct_def,
        );
        assert_eq!(
            t,
            Struct {
                name: arcstr::literal!("some_struct"),
                is_large_data: false,
                preferred_transfer_mode: None,
                fields: vec![
                    StructField {
                        name: arcstr::literal!("bla"),
                        ty: NominalType::Int(IntType::U8),
                        location: Span::default(),
                    },
                    StructField {
                        name: arcstr::literal!("buffer"),
                        ty: NominalType::Bytes {
                            size: 0x100,
                            alignment: 0x1,
                        },
                        location: Span::default(),
                    },
                ],
                location: Span::default(),
            }
        );
    }

    #[test]
    fn struct_def_marked() {
        let t: Struct = unwrap_parse(
            r"
                struct some_struct : sf::LargeData, sf::PrefersPointerTransferMode {
                    u8 bla;
                    sf::Bytes<0x100> buffer;
                };",
            parse_struct_def,
        );
        assert_eq!(
            t,
            Struct {
                name: arcstr::literal!("some_struct"),
                is_large_data: true,
                preferred_transfer_mode: Some(BufferTransferMode::Pointer),
                fields: vec![
                    StructField {
                        name: arcstr::literal!("bla"),
                        ty: NominalType::Int(IntType::U8),
                        location: Span::default(),
                    },
                    StructField {
                        name: arcstr::literal!("buffer"),
                        ty: NominalType::Bytes {
                            size: 0x100,
                            alignment: 0x1,
                        },
                        location: Span::default(),
                    },
                ],
                location: Span::default(),
            }
        );
    }

    fn parse_interface(s: &str) -> Result<Interface, ParseError> {
        parser::InterfaceDefParser::new().parse(0, s)
    }

    #[test]
    fn idirectory_interface() {
        let s = r#"
interface fssrv::sf::IDirectory {
	# Takes a type-0x6 output buffer. Returns an output u64(?) for the total
	# number of read entries, this is 0 when no more entries are available.
	# 
	# The output buffer contains the read array of
	# [\#DirectoryEntry](http://switchbrew.org/index.php?title=Filesystem%20services#DirectoryEntry "wikilink").
	# This doesn't include entries for "." and "..".
	# 
	[0] Read(sf::Out<s64> out, sf::OutBuffer out_entries);
	# Returns an u64 for the total number of readable entries.
	# 
	[1] GetEntryCount(sf::Out<s64> out);
}
        "#;
        let interface: Interface = unwrap_parse(s, parse_interface);

        println!("{:#?}", interface);
    }

    #[test]
    fn iuserinterface_interface() {
        let s = r#"
interface sm::detail::IUserInterface is sm: {
	# Needs to be called before any other command may be used. On version 3.0.0
	# and lower, if this function is not called, `GetService`, `RegisterService`
	# and `UnregisterService` may be called without restriction, thanks to
	# `sm:h`.
	#
	# # Arguments
	# - `reserved`:  Should be set to 0.
	[0] Initialize(sf::ClientProcessId);
	# Returns a handle to the given service. IPC messages may be sent to this
	# handle through `svcSendSyncRequest`.
	[1] GetService(ServiceName name, OutMoveHandle session_handle);
	# Registers a service with the given name. The user can use
	# `svcAcceptSession` on the returned handle to get a new Session handle, and
	# use `svcReplyAndReceive` on those handles to reply to IPC requests.
	[2] RegisterService(ServiceName name, u8, u32 maxHandles, OutMoveHandle port_handle);
	# Unregisters the given service. Future `GetService` call will not return
	# this service anymore, but existing handles will stay alive.
	[3] UnregisterService(ServiceName name);
}
        "#;
        let interface: Interface = unwrap_parse(s, parse_interface);

        println!("{:#?}", interface);
    }

    fn parse_ipc_file(s: &str) -> Result<IpcFile, ParseError> {
        parser::IpcFileParser::new().parse(0, s)
    }

    #[test]
    fn multiple_def_file() {
        let s = r#"
type a = u8;
type b = u8;
type a = u8;
struct c {};
struct a {};
struct b {};
type c = a;
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Multiple definitions of type `a`");
    }

    #[test]
    fn undef_alias_file() {
        let s = r#"
type t = undefined_type;
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Could not resolve type named `undefined_type`",
        );
    }

    #[test]
    fn undef_struct_file() {
        let s = r#"
struct s {
    undefined_type value;
};
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Could not resolve type named `undefined_type`",
        );
    }

    #[test]
    fn unsized_struct_file() {
        let s = r#"
struct test {
    sf::Unknown<1> sized_value;
    sf::Unknown  unsized_value;
};
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Use of unsized type in field `unsized_value`",
        );
    }

    #[test]
    fn enum_overflow_file() {
        let s = r#"
enum test : u8 {
    ok1 = 1,
    large = 256,
    ok2 = 2,
};
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Value 256 of enum arm `large` does not fit into type U8",
        );
    }

    #[test]
    fn enum_duplicate_val_file() {
        let s = r#"
enum test : u8 {
    one_1 = 1,
    one_2 = 1,
    two_1 = 2,
    one_3 = 1,
    two_2 = 2,
};
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Duplicate enum value");
    }

    #[test]
    fn enum_duplicate_name_file() {
        let s = r#"
enum test : u8 {
    name = 1,
    name = 2,
};
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Duplicate enum arm named `name`");
    }

    #[test]
    fn struct_duplicate_file() {
        let s = r#"
struct test {
    u8 one;
    u16 one;
};
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Duplicate struct field `one`");
    }

    #[test]
    fn bitflags_overflow_file() {
        let s = r#"
bitflags test : u8 {
    ok1 = 1,
    large = 256,
    ok2 = 1,
};
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Value 256 of bitflags arm `large` does not fit into type U8",
        );
    }

    #[test]
    fn bitflags_duplicate_file() {
        let s = r#"
bitflags test : u8 {
    one = 1,
    one = 2,
};
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Duplicate bitfield arm named `one`");
    }

    #[test]
    fn interface_duplicate_id_file() {
        let s = r#"
interface ITest {
    [1] Lol();
    [1] Kek();
}
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Duplicate command with id `1`");
    }

    #[test]
    fn interface_duplicate_name_file() {
        let s = r#"
interface ITest {
    [1] Lol();
    [2] Lol();
}
        "#;
        unwrap_err_parse(s, parse_ipc_file, "Duplicate command named `Lol`");
    }

    #[test]
    fn interface_undef_type_file() {
        let s = r#"
interface ITest {
    [1] Lol(undefined_type hello);
}
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Could not resolve type named `undefined_type`",
        );
    }

    #[test]
    fn interface_undef_interface_file() {
        let s = r#"
interface ITest {
    [1] Lol(sf::SharedPointer<ISomeUndefinedInterface> hello);
}
        "#;
        unwrap_err_parse(
            s,
            parse_ipc_file,
            "Could not resolve interface named `ISomeUndefinedInterface`",
        );
    }
}
