//! This implements a parser and a data model for (a variant of) [SwIPC](https://github.com/reswitched/SwIPC) files
//!
//! Notable changes:
//! - No more typed buffers, only bytes. Use either LargeData marker for the struct (TODO) or an array
//!    So, no more `buffer<data_type, transfer_type, size>`, only `buffer<transfer_type, size>` or `buffer<transfer_type>`
//! - Allow (and prefer) symbolic names for buffer transfer types
//!     
//!

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use lalrpop_util::lexer::Token;
use lalrpop_util::{lalrpop_mod, ParseError};

pub mod model;

lalrpop_mod!(parser, "/swipc/swipc.rs");

fn make_diagnostic<'source>(
    source: &'source str,
    error: ParseError<usize, Token<'source>, &'static str>,
) -> (SimpleFiles<&'source str, &'source str>, Diagnostic<usize>) {
    let mut files = SimpleFiles::new();
    let file_id = files.add("/dev/stdin", source);

    let diagnostic = Diagnostic::error();

    let diagnostic = match error {
        ParseError::InvalidToken { location } => diagnostic
            .with_message("Invalid token")
            .with_labels(vec![Label::primary(file_id, location..location)]),
        ParseError::UnrecognizedEOF { location, expected } => diagnostic
            .with_message("Unrecognized EOF")
            .with_labels(vec![Label::primary(file_id, location..location)])
            .with_notes(vec![format!(
                "Expected one of the following: {}",
                expected.join(", ")
            )]),
        ParseError::UnrecognizedToken {
            token: (start, t, end),
            expected,
        } => diagnostic
            .with_message(format!("Unrecognized token: {}", t))
            .with_labels(vec![Label::primary(file_id, start..end)])
            .with_notes(vec![format!(
                "Expected one of the following: {}",
                expected.join(", ")
            )]),
        ParseError::ExtraToken {
            token: (start, t, end),
        } => diagnostic
            .with_message(format!("Extra token: {}", t))
            .with_labels(vec![Label::primary(file_id, start..end)]),
        ParseError::User { .. } => {
            unreachable!()
        }
    };

    (files, diagnostic)
}

#[cfg(test)]
mod tests {
    use crate::swipc::model::{IntType, Interface, Struct, Type};
    use crate::swipc::{make_diagnostic, parser};
    use arcstr::ArcStr;
    use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
    use lalrpop_util::lexer::Token;
    use lalrpop_util::ParseError;

    fn unwrap_parse<T>(
        source: &str,
        parser: impl FnOnce(&str) -> Result<T, ParseError<usize, Token<'_>, &'static str>>,
    ) -> T {
        match parser(source) {
            Ok(r) => r,
            Err(error) => {
                let (files, diagnostic) = make_diagnostic(source, error);

                let writer = StandardStream::stderr(ColorChoice::Always);
                let config = codespan_reporting::term::Config::default();

                codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diagnostic)
                    .unwrap();

                panic!("Parse error occurred");
            }
        }
    }

    fn parse_typedef(
        s: &str,
    ) -> Result<(ArcStr, Type), ParseError<usize, Token<'_>, &'static str>> {
        parser::TypedefParser::new().parse(s)
    }

    #[test]
    fn simple_typedef() {
        let t: (ArcStr, Type) = unwrap_parse("type hello::world = u8;", parse_typedef);
        assert_eq!(t.0, "hello::world");
        assert_eq!(t.1, Type::Int(IntType::U8));
    }

    #[test]
    fn struct_typedef() {
        let t: (ArcStr, Type) = unwrap_parse(
            r"type some_struct = struct {
                    u8 bla;
                    bytes<0x100> buffer_;
                };",
            parse_typedef,
        );
        assert_eq!(t.0, "some_struct");
        assert_eq!(
            t.1,
            Type::Struct(Struct {
                fields: vec![
                    (arcstr::literal!("bla"), Type::Int(IntType::U8)),
                    (
                        arcstr::literal!("buffer_"),
                        Type::Bytes {
                            size: 0x100,
                            alignment: 0x1,
                        }
                    ),
                ]
            })
        );
    }

    fn parse_interface(s: &str) -> Result<Interface, ParseError<usize, Token<'_>, &'static str>> {
        parser::InterfaceParser::new().parse(s)
    }

    #[test]
    fn idirectory_interface() {
        let s = r#"
interface nn::fssrv::sf::IDirectory {
	# Takes a type-0x6 output buffer. Returns an output u64(?) for the total
	# number of read entries, this is 0 when no more entries are available.
	# 
	# The output buffer contains the read array of
	# [\#DirectoryEntry](http://switchbrew.org/index.php?title=Filesystem%20services#DirectoryEntry "wikilink").
	# This doesn't include entries for "." and "..".
	# 
	[0] Read() -> (u64, array<nn::fssrv::sf::IDirectoryEntry, 6>);
	# Returns an u64 for the total number of readable entries.
	# 
	[1] GetEntryCount() -> u64;
}
        "#;
        let interface: Interface = unwrap_parse(s, parse_interface);

        println!("{:#?}", interface);
    }

    #[test]
    fn ifilesystemproxy_interface() {
        let s = r#"
interface nn::fssrv::sf::IFileSystemProxy is fsp-srv {
	# Takes an input
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# and an u64 title-id. Returns an
	# [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# \[2.0.0+\] This function was removed.
	# 
	@version(1.0.0)
	[0] OpenFileSystem(nn::fssrv::sf::FileSystemType filesystem_type, buffer<0x19, 0x301>) -> object<nn::fssrv::sf::IFileSystem>;
	# Takes a pid-descriptor.
	# 
	[1] SetCurrentProcess(u64, pid);
	[2] OpenDataFileSystemByCurrentProcess() -> object<nn::fssrv::sf::IFileSystem>;
	# Takes an input
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# and an u64 title-id. Returns an
	# [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# Web-applet loads the
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# (which must be **ContentManual**) from u32\_table\[inparam\].
	# 
	# Note: web-applet strings refer to both this cmd and
	# [\#OpenFileSystemWithId](#nn::fssrv::sf::IFileSystemProxy\(8\) "wikilink")
	# as "MountContent", but official nn\_sf\_sync symbols use "OpenXX" names.
	# 
	@version(2.0.0+)
	[7] OpenFileSystemWithPatch(nn::fssrv::sf::FileSystemType filesystem_type, nn:ApplicationId id) -> object<nn::fssrv::sf::IFileSystem>;
	# Takes a type-0x19 input buffer, an
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# and an u64 title-id. Returns an
	# [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# The [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink") must be
	# **ContentMeta** if the NCA type is 0 (control).
	# 
	# The input buffer is the output string path from
	# [GetApplicationContentPath](http://switchbrew.org/index.php?title=NS_Services#GetApplicationContentPath "wikilink").
	# 
	# May return errors when attempting to access NCA-paths for an
	# update-title with a gamecard, when the gamecard isn't inserted. May
	# return error 0x7D402 in some cases with update-titles. Non-val2 in32
	# values with NCA-type1 are unusable, even for normal titles.
	# 
	# The official "MountApplicationPackage" func uses this with in64=0 and
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# **ApplicationPackage**.
	# 
	# After the
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# specific permissions are checked, it then gets the func retval for
	# permissions-type 0x25 and func0.
	# 
	# When
	# [\#FileSystemType](http://switchbrew.org/index.php?title=Filesystem%20services#FileSystemType "wikilink")
	# is **ContentMeta**, it uses in64=0xffffffffffffffff internally,
	# otherwise it checks if in64 is set to 0xffffffffffffffff then throws an
	# error if so. When the in64 used internally is not 0xffffffffffffffff,
	# it's compared with the NCA titleID, then an error is thrown on mismatch.
	# 
	@version(2.0.0+)
	[8] OpenFileSystemWithId(nn::fssrv::sf::FileSystemType filesystem_type, nn::ApplicationId tid, buffer<0x19,0x301>) -> object<nn::fssrv::sf::IFileSystem> contentFs;
	@version(3.0.0+)
	[9] OpenDataFileSystemByApplicationId(nn::ApplicationId tid) -> object<nn::fssrv::sf::IFileSystem>;
	# Takes a type-0x19 input buffer string and a u32 [Bis
	# partitionID](http://switchbrew.org/index.php?title=Flash_Filesystem "wikilink").
	# Official user-process code sets instr\[0\] = 0 normally. Returns an
	# [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# Only partitionIDs for FAT partitions are usable with this, otherwise
	# error 0x2EE202 is returned. Seems to be about the same as
	# [\#OpenBisStorage](#nn::fssrv::sf::IFileSystemProxy\(12\) "wikilink")
	# except this mounts the partition filesystem instead of allowing direct
	# access to the partition sectors.
	# 
	[11] OpenBisFileSystem(nn::fssrv::sf::Partition partitionId, buffer<0x19,0x301>) -> object<nn::fssrv::sf::IFileSystem> bis;
	# Takes a u32 partition ID, returns 0x2EE202 for partitions which do not
	# exist, 0x320002 for partitions which cannot be opened and a valid
	# [\#IStorage](#nn::fssrv::sf::IStorage "wikilink") handle otherwise.
	# 
	[12] OpenBisStorage(nn::fs::sf::Partition partitionId) -> object<nn::fssrv::sf::IStorage> bisPartition;
	[13] InvalidateBisCache();
	[17] OpenHostFileSystem(buffer<0x19,0x301>) -> object<nn::fssrv::sf::IFileSystem>;
	[18] OpenSdCardFileSystem() -> object<nn::fssrv::sf::IFileSystem>;
	@version(2.0.0+)
	[19] FormatSdCardFileSystem();
	# Takes an input u64.
	# 
	[21] DeleteSaveDataFileSystem(nn::ApplicationId tid);
	# Takes a 0x40-byte Save-struct entry, a 0x40-byte SaveCreate-struct
	# entry, and a 0x10-byte input struct.
	# 
	# Only the first 0x5-bytes in the 0x10-byte struct are initialized:
	# all-zero when automatically creating savedata during savecommon mount by
	# official user-processes. In the dedicated save-creation code in official
	# user-processes: +0 u32 = 0x40060, +4 u8 = 1.
	# 
	# Creates regular savedata.
	# 
	[22] CreateSaveDataFileSystem(nn::fssrv::sf::SaveStruct save_struct, nn::fssrv::sf::SaveCreateStruct ave_create_struct, bytes<0x10,4>);
	# Takes a 0x40-byte Save-struct entry and a 0x40-byte SaveCreate-struct
	# entry.
	# 
	# Creates savedata in the SYSTEM
	# [NAND](http://switchbrew.org/index.php?title=Flash_Filesystem "wikilink")
	# partition.
	# 
	[23] CreateSaveDataFileSystemBySystemSaveDataId(nn::fssrv::sf::SaveStruct, nn::fssrv::sf::SaveCreateStruct);
	[24] RegisterSaveDataFileSystemAtomicDeletion(buffer<5>);
	@version(2.0.0+)
	[25] DeleteSaveDataFileSystemBySaveDataSpaceId(u8, u64);
	@version(2.0.0+)
	[26] FormatSdCardDryRun();
	@version(2.0.0+)
	[27] IsExFatSupported() -> b8;
	@version(4.0.0+)
	[28] DeleteSaveDataFileSystemBySaveDataAttribute(u8, bytes<0x40,8>);
	# Takes two input u32s (gamecard handle, partition ID), and returns an
	# [\#IStorage](#nn::fssrv::sf::IStorage "wikilink") for the
	# [partition](http://switchbrew.org/index.php?title=Gamecard_Format "wikilink").
	# 
	[30] OpenGameCardStorage(u32, u32) -> object<nn::fssrv::sf::IStorage>;
	# Takes two input u32s, with the second u32 located at +4 in rawdata after
	# the first u32. Returns an
	# [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# Mounts a [gamecard
	# partition](http://switchbrew.org/index.php?title=Gamecard_Partition "wikilink").
	# 
	[31] OpenGameCardFileSystem(u32, u32) -> object<nn::fssrv::sf::IFileSystem>;
	@version(3.0.0+)
	[32] ExtendSaveDataFileSystem(u8, u64, u64, u64);
	@version(5.0.0+)
	@undocumented
	[33] DeleteCacheStorage(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[34] GetCacheStorageSize(unknown) -> unknown;
	# Takes an input u8
	# [\#SaveDataSpaceId](http://switchbrew.org/index.php?title=Filesystem%20services#SaveDataSpaceId "wikilink")
	# and a 0x40-byte Save-struct entry. Official user-process code is only
	# known to use value 1 for the u8.
	# 
	# Returns an [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# Permissions aren't checked until the specified save is successfully
	# found.
	# 
	# Only one process (specifically only one
	# [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink") session) can
	# mount a given savedata at any given time (this includes SystemSaveData).
	# 
	[51] OpenSaveDataFileSystem(u8 save_data_space_id, nn::fssrv::sf::SaveStruct save_struct) -> object<nn::fssrv::sf::IFileSystem>;
	# Takes an input u8
	# [\#SaveDataSpaceId](http://switchbrew.org/index.php?title=Filesystem%20services#SaveDataSpaceId "wikilink")
	# and a 0x40-byte Save-struct entry. Web-applet only uses value0 for the
	# input u8.
	# 
	# Returns an [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink").
	# 
	# Mounts savedata in the SYSTEM
	# [NAND](http://switchbrew.org/index.php?title=Flash_Filesystem "wikilink")
	# partition.
	# 
	[52] OpenSaveDataFileSystemBySystemSaveDataId(u8 save_data_space_id, nn::fssrv::sf::SaveStruct save_struct) -> object<nn::fssrv::sf::IFileSystem>;
	@version(2.0.0+)
	[53] OpenReadOnlySaveDataFileSystem(u8 save_data_space_id, nn::fssrv::sf::SaveStruct save_struct) -> object<nn::fssrv::sf::IFileSystem>;
	@version(3.0.0+)
	[57] ReadSaveDataFileSystemExtraDataBySaveDataSpaceId(u8, u64) -> buffer<6>;
	[58] ReadSaveDataFileSystemExtraData(u64) -> buffer<6>;
	@version(2.0.0+)
	[59] WriteSaveDataFileSystemExtraData(u8, u64, buffer<5>);
	# No input, returns an output
	# [\#ISaveDataInfoReader](#nn::fssrv::sf::ISaveDataInfoReader "wikilink").
	# 
	[60] OpenSaveDataInfoReader() -> object<nn::fssrv::sf::ISaveDataInfoReader>;
	# Takes an input u8
	# [\#SaveDataSpaceId](http://switchbrew.org/index.php?title=Filesystem%20services#SaveDataSpaceId "wikilink"),
	# returns an output
	# [\#ISaveDataInfoReader](#nn::fssrv::sf::ISaveDataInfoReader "wikilink").
	# 
	[61] OpenSaveDataInfoReaderBySaveDataSpaceId(u8) -> object<nn::fssrv::sf::ISaveDataInfoReader>;
	@version(5.0.0+)
	@undocumented
	[62] OpenCacheStorageList(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[64] OpenSaveDataInternalStorageFileSystem(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[65] UpdateSaveDataMacForDebug(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[66] WriteSaveDataFileSystemExtraData2(unknown) -> unknown;
	[80] OpenSaveDataMetaFile(u8, u32, bytes<0x40,8>) -> object<nn::fssrv::sf::IFile>;
	@version(4.0.0+)
	[81] OpenSaveDataTransferManager() -> object<nn::fssrv::sf::ISaveDataTransferManager>;
	@version(5.0.0+)
	@undocumented
	[82] OpenSaveDataTransferManagerVersion2(unknown) -> unknown;
	[100] OpenImageDirectoryFileSystem(u32) -> object<nn::fssrv::sf::IFileSystem>;
	# Takes a
	# [\#ContentStorageId](http://switchbrew.org/index.php?title=Filesystem%20services#ContentStorageId "wikilink").
	# Invalid values return 0x2EE202.
	# 
	# Returns an [\#IFileSystem](#nn::fssrv::sf::IFileSystem "wikilink") with
	# NCA files. The read data from these files is identical to the data read
	# by
	# [Content\_Manager\_services\#ReadEntryRaw](http://switchbrew.org/index.php?title=Content_Manager_services#ReadEntryRaw "wikilink").
	# 
	[110] OpenContentStorageFileSystem(u32 content_storage_id) -> object<nn::fssrv::sf::IFileSystem> content_fs;
	[200] OpenDataStorageByCurrentProcess() -> object<nn::fssrv::sf::IStorage> data_storage;
	@version(3.0.0+)
	[201] OpenDataStorageByProgramId(nn::ApplicationId tid) -> object<nn::fssrv::sf::IStorage> data_storage;
	# Takes a
	# [\#StorageId](http://switchbrew.org/index.php?title=Filesystem%20services#StorageId "wikilink")
	# and a TitleID.
	# 
	# Returns a [domain object
	# ID](http://switchbrew.org/index.php?title=IPC_Marshalling#Domain_message "wikilink")
	# implementing the [\#IStorage](#nn::fssrv::sf::IStorage "wikilink")
	# interface for data archives.
	# 
	[202] OpenDataStorageByDataId(u8 storage_id, nn::ApplicationId tid) -> object<nn::fssrv::sf::IStorage> data_storage;
	[203] OpenPatchDataStorageByCurrentProcess() -> object<nn::fssrv::sf::IStorage>;
	# This command returns a session to a port implementing the
	# [\#IDeviceOperator](#nn::fssrv::sf::IDeviceOperator "wikilink")
	# interface.
	# 
	[400] OpenDeviceOperator() -> object<nn::fssrv::sf::IDeviceOperator>;
	# This command returns a session to a port implementing the
	# [\#IEventNotifier](#nn::fssrv::sf::IEventNotifier "wikilink") interface.
	# 
	[500] OpenSdCardDetectionEventNotifier() -> object<nn::fssrv::sf::IEventNotifier> sd_event_notify;
	# This command returns a session to a port implementing the
	# [\#IEventNotifier](#nn::fssrv::sf::IEventNotifier "wikilink") interface.
	# 
	[501] OpenGameCardDetectionEventNotifier() -> object<nn::fssrv::sf::IEventNotifier> game_card_event_notify;
	@version(5.0.0+)
	@undocumented
	[510] OpenSystemDataUpdateEventNotifier(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[511] NotifySystemDataUpdateEvent(unknown) -> unknown;
	@version(1.0.0-3.0.2)
	[600] SetCurrentPosixTime(u64 time);
	[601] QuerySaveDataTotalSize(u64, u64) -> u64 save_data_size;
	# Takes an unknown input u64 and a type-0x6 output buffer.
	# 
	# The input u64 high-byte must be non-zero, otherwise an
	# [error](http://switchbrew.org/index.php?title=Error_codes "wikilink") is
	# returned(0xE02).
	# 
	[602] VerifySaveDataFileSystem(u64) -> buffer<6>;
	[603] CorruptSaveDataFileSystem(u64);
	[604] CreatePaddingFile(u64);
	[605] DeleteAllPaddingFiles();
	@version(2.0.0+)
	[606] GetRightsId(u8, u64) -> bytes<0x10,8> rights;
	@version(2.0.0+)
	[607] RegisterExternalKey(bytes<0x10,8>, bytes<0x10,1>);
	@version(2.0.0+)
	[608] UnregisterAllExternalKey();
	@version(2.0.0+)
	[609] GetRightsIdByPath(buffer<0x19,0x301>) -> bytes<0x10,8> rights;
	@version(3.0.0+)
	[610] GetRightsIdAndKeyGenerationByPath(buffer<0x19,0x301>) -> (u8, bytes<0x10,8> rights);
	@version(4.0.0+)
	[611] SetCurrentPosixTimeWithTimeDifference(u32, u64);
	@version(4.0.0+)
	[612] GetFreeSpaceSizeForSaveData(u8) -> u64;
	@version(4.0.0+)
	[613] VerifySaveDataFileSystemBySaveDataSpaceId(u8, u64) -> buffer<6>;
	@version(4.0.0+)
	[614] CorruptSaveDataFileSystemBySaveDataSpaceId(u8, u64);
	@version(5.0.0+)
	@undocumented
	[615] QuerySaveDataInternalStorageTotalSize(unknown) -> unknown;
	# Takes in the 0x10 byte SD card encryption seed, and loads it into
	# FS-module
	# state.
	# 
	# [NS](http://switchbrew.org/index.php?title=NS_Services "wikilink")-module
	# reads the 0x10 bytes from SdCard:/Nintendo/Contents/private, and
	# compares them to the first 0x10 bytes of the ns\_appman:/private (in
	# [system
	# savedata](http://switchbrew.org/index.php?title=Flash_Filesystem#System_Savegames "wikilink")
	# 0x8000000000000043). If they match, NS calls this command using bytes
	# 0x10-0x20 from ns\_appman:/private. The rest of this file (0x1F0 bytes
	# total) is (usually/always?) all-zero.
	# 
	@version(2.0.0+)
	[620] SetSdCardEncryptionSeed(bytes<0x10,1>);
	@version(4.0.0+)
	[630] SetSdCardAccessibility(u8);
	@version(4.0.0+)
	[631] IsSdCardAccessible() -> u8;
	@version(4.0.0+)
	[640] IsSignedSystemPartitionOnSdCardValid() -> u8;
	@version(5.0.0+)
	@undocumented
	[700] OpenAccessFailureResolver(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[701] GetAccessFailureDetectionEvent(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[702] IsAccessFailureDetected(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[710] ResolveAccessFailure(unknown) -> unknown;
	@version(5.0.0+)
	@undocumented
	[720] AbandonAccessFailure(unknown) -> unknown;
	@version(2.0.0+)
	[800] GetAndClearFileSystemProxyErrorInfo() -> bytes<0x80,4> error_info;
	[1000] SetBisRootForHost(u32, buffer<0x19,0x301>);
	[1001] SetSaveDataSize(u64, u64);
	[1002] SetSaveDataRootPath(buffer<0x19,0x301>);
	[1003] DisableAutoSaveDataCreation();
	# Takes an input u32.
	# 
	[1004] SetGlobalAccessLogMode(u32 mode);
	# Returns an output u32.
	# 
	# GlobalAccessLogMode is normally 0.
	# 
	[1005] GetGlobalAccessLogMode() -> u32 mode;
	# Takes a type-0x5 input buffer.
	# 
	# The input buffer is the string to output to the log. User-processes
	# normally include a newline at the end.
	# 
	# User-processes only use this when the value previously loaded from
	# [\#GetGlobalAccessLogMode](#nn::fssrv::sf::IFileSystemProxy\(1005\) "wikilink")
	# has bit1 set.
	# 
	# When bit1 in GlobalAccessLogMode is clear, FS-module will just return 0
	# for OutputAccessLogToSdCard. However even with that set the log doesn't
	# show up SD, unknown why.
	# 
	# The input buffer is written to the "$FsAccessLog:/FsAccessLog.txt" file,
	# where "$FsAccessLog" is the SD-card mount-name. It's written to the
	# current end of the file(appended).
	# 
	[1006] OutputAccessLogToSdCard(buffer<5> log_text);
	@version(4.0.0+)
	[1007] RegisterUpdatePartition();
	@version(4.0.0+)
	[1008] OpenRegisteredUpdatePartition() -> object<nn::fssrv::sf::IFileSystem>;
	@version(4.0.0+)
	[1009] GetAndClearMemoryReportInfo() -> bytes<0x80,8>;
	@version(5.1.0+)
	@undocumented
	[1010] Unknown1010(unknown) -> unknown;
	@version(4.0.0+)
	[1100] OverrideSaveDataTransferTokenSignVerificationKey(buffer<5>);
}
        "#;
        let interface: Interface = unwrap_parse(s, parse_interface);

        println!("{:#?}", interface);
    }

    #[test]
    fn iuserinterface_interface() {
        let s = r#"
interface nn::sm::detail::IUserInterface is sm: {
	# Needs to be called before any other command may be used. On version 3.0.0
	# and lower, if this function is not called, `GetService`, `RegisterService`
	# and `UnregisterService` may be called without restriction, thanks to
	# `sm:h`.
	#
	# # Arguments
	# - `reserved`:  Should be set to 0.
	[0] Initialize(pid, u64 reserved);
	# Returns a handle to the given service. IPC messages may be sent to this
	# handle through `svcSendSyncRequest`.
	[1] GetService(ServiceName name) -> handle<move, session>;
	# Registers a service with the given name. The user can use
	# `svcAcceptSession` on the returned handle to get a new Session handle, and
	# use `svcReplyAndReceive` on those handles to reply to IPC requests.
	[2] RegisterService(ServiceName name, u8, u32 maxHandles) -> handle<move, port>;
	# Unregisters the given service. Future `GetService` call will not return
	# this service anymore, but existing handles will stay alive.
	[3] UnregisterService(ServiceName name);
}
        "#;
        let interface: Interface = unwrap_parse(s, parse_interface);

        println!("{:#?}", interface);
    }
}
