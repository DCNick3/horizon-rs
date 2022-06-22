use anyhow::Context;
use once_cell::sync::Lazy;
use py_literal::Value;
use regex::{Regex, RegexBuilder};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

static PY_COMMENT_REGEX: Lazy<Regex> =
    Lazy::new(|| RegexBuilder::new("#.*$").multi_line(true).build().unwrap());

#[derive(Debug)]
pub struct IpcFile {
    // there's also potentially useful info
    pub name: String,
    pub interfaces: Vec<IpcInterface>,
}

impl IpcFile {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        // the whole file is not a valid python syntax, but rather being an entry in a dict
        // so we wrap it in braces {} for it to be a valid dict
        let s = format!("{{{}}}", s);

        let s = PY_COMMENT_REGEX.replace_all(&s, "");

        // py_literal does not support newlines, sooooo
        let s = s.replace("\n", " ");

        let lit = Value::from_str(&s).context("Parsing IpcFile as a python literal")?;

        let lit = lit.as_dict().unwrap().first().unwrap();

        Self::from_pyliteral(lit)
    }

    pub fn from_pyliteral((name, lit): &(Value, Value)) -> anyhow::Result<Self> {
        let name = name.as_string().unwrap().clone();

        let interfaces = lit
            .as_dict()
            .unwrap()
            .iter()
            .map(|kv| IpcInterface::from_pyliteral(kv).unwrap())
            .collect::<Vec<_>>();

        Ok(Self { name, interfaces })
    }
}

#[derive(Debug)]
pub struct IpcInterface {
    // there's also potentially useful info
    pub raw_name: String,
    pub methods: BTreeMap<u32, IpcMethod>,
}

impl IpcInterface {
    pub fn from_pyliteral((raw_name, lit): &(Value, Value)) -> anyhow::Result<Self> {
        let raw_name = raw_name.as_string().unwrap().clone();

        let methods = lit
            .as_dict()
            .unwrap()
            .iter()
            .map(|(k, v)| -> (u32, _) {
                (
                    k.as_integer().unwrap().try_into().unwrap(),
                    IpcMethod::from_pyliteral(v).unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        Ok(Self { raw_name, methods })
    }
}

#[derive(Debug)]
pub struct IpcMethod {
    pub virtual_table_offset: u32,
    /// Probably just an address of the implementation?
    pub function_address: Option<u64>,
    /// ???
    pub lr_value: u64,
    /// How many raw payload bytes does the function take
    pub in_bytes: u32,
    /// How many raw payload bytes does the function return
    pub out_bytes: u32,
    /// Whether the client should use "send pid" feature of CMIF to send its PID to the server
    pub pid: bool,
    /// Types of buffers used by the function
    pub buffers: Vec<u32>,

    pub in_interfaces: Vec<String>,
    pub out_interfaces: Vec<Option<String>>,

    pub in_handles: Vec<u32>,
    pub out_handles: Vec<u32>,
}

impl IpcMethod {
    pub fn from_pyliteral(lit: &Value) -> anyhow::Result<Self> {
        let lit = lit
            .as_dict()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.as_string().unwrap().clone(), v.clone()))
            .collect::<HashMap<_, _>>();

        // oh god, this is ugly
        // TODO: error handling
        // TODO: maybe write a serde-based deserialized for pyliteral stuff?

        Ok(Self {
            virtual_table_offset: lit
                .get("vt")
                .unwrap()
                .as_integer()
                .unwrap()
                .try_into()
                .unwrap(),
            function_address: lit
                .get("func")
                .map(|v| v.as_integer().unwrap().try_into().unwrap()),
            lr_value: lit
                .get("lr")
                .unwrap()
                .as_integer()
                .unwrap()
                .try_into()
                .unwrap(),
            in_bytes: lit
                .get("inbytes")
                .unwrap()
                .as_integer()
                .unwrap()
                .try_into()
                .unwrap(),
            out_bytes: lit
                .get("outbytes")
                .unwrap()
                .as_integer()
                .unwrap()
                .try_into()
                .unwrap(),
            pid: lit
                .get("pid")
                .map(|v| v.as_boolean().unwrap())
                .unwrap_or(false),
            buffers: lit
                .get("buffers")
                .map(|v| {
                    v.as_list()
                        .unwrap()
                        .iter()
                        .map(|v| -> u32 { v.as_integer().unwrap().try_into().unwrap() })
                        .collect()
                })
                .unwrap_or_default(),
            in_interfaces: lit
                .get("ininterfaces")
                .map(|v| {
                    v.as_list()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_string().unwrap().clone())
                        .collect()
                })
                .unwrap_or_default(),
            out_interfaces: lit
                .get("outinterfaces")
                .map(|v| {
                    v.as_list()
                        .unwrap()
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            Value::None => None,
                            _ => panic!("Unsupported out_interfaces type"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            in_handles: lit
                .get("inhandles")
                .map(|v| {
                    v.as_list()
                        .unwrap()
                        .iter()
                        .map(|v| -> u32 { v.as_integer().unwrap().try_into().unwrap() })
                        .collect()
                })
                .unwrap_or_default(),
            out_handles: lit
                .get("outhandles")
                .map(|v| {
                    v.as_list()
                        .unwrap()
                        .iter()
                        .map(|v| -> u32 { v.as_integer().unwrap().try_into().unwrap() })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let lit = Value::from_str(s).context("Parsing IpcMethod as a python literal")?;

        Self::from_pyliteral(&lit)
    }
}

#[cfg(test)]
mod tests {
    use crate::ipc_parse::{IpcFile, IpcMethod};

    fn try_parse_ipc_method(s: &str) {
        match IpcMethod::parse(s) {
            Ok(ipc_method) => {
                println!("{:?}", ipc_method);
            }
            Err(e) => {
                eprintln!("Could not parse {}", s);
                eprintln!("{}", e);
                panic!("it all failed!!");
            }
        }
    }
    fn try_parse_ipc_file(s: &str) {
        match IpcFile::parse(s) {
            Ok(ipc_file) => {
                println!("{:#?}", ipc_file);
            }
            Err(e) => {
                eprintln!("Could not parse {}", s);
                eprintln!("{}", e);
                panic!("it all failed!!");
            }
        }
    }

    #[test]
    fn parse_ipc_method() {
        try_parse_ipc_method(
            r#"{"vt":  0x20, "func": 0x710007C9DC, "lr": 0x71000076C0, "inbytes":     8, "outbytes":     0, "pid": True}"#,
        );
        try_parse_ipc_method(
            r#"{"vt": 0x180, "func": 0x71000A68B4, "lr": 0x71000069E0, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']}"#,
        );
        try_parse_ipc_method(
            r#"{"vt":  0x70, "func": 0x71000AE374, "lr": 0x710000AE48, "inbytes":  0x10, "outbytes":  0x20, "buffers": [5, 5]}"#,
        );
        try_parse_ipc_method(
            r#"{"vt":  0x20, "lr": 0x710000E694, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x710000E81C']}"#,
        );
    }

    #[test]
    fn parse_ipc_file() {
        try_parse_ipc_file(
            r#"
'Bus': {
  '0x710000E508': { # , vtable size 1, possible vtables [0x710007BE10 1, 0x710007DA08 1, 0x7100078410 1, 0x71000783B8 1, 0x710007DB60 1, 0x710007BD48 1]
      0:     {"vt":  0x20, "lr": 0x710000E694, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x710000E81C']},
  },
  '0x710000E81C': { # , vtable size 3, possible vtables [0x710007D5F8 3, 0x710007B578 3, 0x7100078478 3]
      0:     {"vt":  0x20, "lr": 0x710000E9F8, "inbytes":     4, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x710000EB60, "inbytes":     0, "outbytes":     4},
      2:     {"vt":  0x30, "lr": 0x710000ECD8, "inbytes":     4, "outbytes":     0},
  },
  'N2nn2sf22UnmanagedServiceObjectINS0_4hipc6detail12IHipcManagerENS2_6server2v134HipcServerSessionManagerWithDomain15HipcManagerImplEEE': {
      0:     {"vt":  0x20, "func": 0x7100011D7C, "lr": 0x71000128FC, "inbytes":     0, "outbytes":     4},
      1:     {"vt":  0x28, "func": 0x7100011E58, "lr": 0x7100012A84, "inbytes":     4, "outbytes":     0, "outhandles": [2]},
      2:     {"vt":  0x30, "func": 0x7100011E78, "lr": 0x7100012C50, "inbytes":     0, "outbytes":     0, "outhandles": [2]},
      3:     {"vt":  0x38, "func": 0x7100011E94, "lr": 0x7100012DF4, "inbytes":     0, "outbytes":     2},
      4:     {"vt":  0x40, "func": 0x7100011EA8, "lr": 0x7100012F7C, "inbytes":     4, "outbytes":     0, "outhandles": [2]},
  },
  'N2nn2sf22UnmanagedServiceObjectINS_4gpio8IManagerENS2_6server11ManagerImplEEE': {
      0:     {"vt":  0x20, "func": 0x710002756C, "lr": 0x71000278F0, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100028640']},
      1:     {"vt":  0x28, "func": 0x710002759C, "lr": 0x7100027AEC, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100028640']},
      2:     {"vt":  0x30, "func": 0x71000275CC, "lr": 0x7100027CE8, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100028640']},
      6:     {"vt":  0x38, "func": 0x71000275FC, "lr": 0x7100027EDC, "inbytes":     4, "outbytes":     0},
      7:     {"vt":  0x40, "func": 0x7100027618, "lr": 0x710002804C, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x7100028640']},
      8:     {"vt":  0x48, "func": 0x7100027648, "lr": 0x7100028240, "inbytes":     4, "outbytes":     1},
      9:     {"vt":  0x50, "func": 0x7100027664, "lr": 0x71000283D4, "inbytes":     8, "outbytes":     0},
      10:    {"vt":  0x58, "func": 0x7100027684, "lr": 0x7100028544, "inbytes":     8, "outbytes":     0},
  },
  'N2nn2sf6detail38ObjectImplFactoryWithStatefulAllocatorINS0_13InterfaceInfoINS_4gpio11IPadSessionEE4_tABINS6_5_tO2NINS4_6server14PadSessionImplEE4typeES5_E4typeENS0_24StatefulAllocationPolicyINS0_16ExpHeapAllocatorEEEE6ObjectE': {
      0:     {"vt":  0x20, "func": 0x710002AB28, "lr": 0x7100028928, "inbytes":     4, "outbytes":     0},
      1:     {"vt":  0x28, "func": 0x710002AB74, "lr": 0x7100028A90, "inbytes":     0, "outbytes":     4},
      2:     {"vt":  0x30, "func": 0x710002ABBC, "lr": 0x7100028C08, "inbytes":     4, "outbytes":     0},
      3:     {"vt":  0x38, "func": 0x710002ABF0, "lr": 0x7100028D70, "inbytes":     0, "outbytes":     4},
      4:     {"vt":  0x40, "func": 0x710002AC38, "lr": 0x7100028EE8, "inbytes":     1, "outbytes":     0},
      5:     {"vt":  0x48, "func": 0x710002AC60, "lr": 0x7100029054, "inbytes":     0, "outbytes":     1},
      6:     {"vt":  0x50, "func": 0x710002ACA8, "lr": 0x71000291CC, "inbytes":     0, "outbytes":     4},
      7:     {"vt":  0x58, "func": 0x710002ACF0, "lr": 0x7100029340, "inbytes":     0, "outbytes":     0},
      8:     {"vt":  0x60, "func": 0x710002AD14, "lr": 0x7100029494, "inbytes":     4, "outbytes":     0},
      9:     {"vt":  0x68, "func": 0x710002AD48, "lr": 0x71000295FC, "inbytes":     0, "outbytes":     4},
      10:    {"vt":  0x70, "func": 0x710002AD90, "lr": 0x710002977C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      11:    {"vt":  0x78, "func": 0x710002ADF8, "lr": 0x710002991C, "inbytes":     0, "outbytes":     0},
      12:    {"vt":  0x80, "func": 0x710002AE1C, "lr": 0x7100029A70, "inbytes":     1, "outbytes":     0},
      13:    {"vt":  0x88, "func": 0x710002AE44, "lr": 0x7100029BDC, "inbytes":     0, "outbytes":     1},
      14:    {"vt":  0x90, "func": 0x710002AE8C, "lr": 0x7100029D54, "inbytes":     4, "outbytes":     0},
      15:    {"vt":  0x98, "func": 0x710002AEB0, "lr": 0x7100029EBC, "inbytes":     0, "outbytes":     4},
      16:    {"vt":  0xA0, "func": 0x710002AEF8, "lr": 0x710002A034, "inbytes":     4, "outbytes":     0},
      17:    {"vt":  0xA8, "func": 0x710002AF2C, "lr": 0x710002A19C, "inbytes":     0, "outbytes":     4},
      18:    {"vt":  0xB0, "func": 0x710002AF74, "lr": 0x710002A310, "inbytes":     0, "outbytes":     0},
      19:    {"vt":  0xB8, "func": 0x710002AF98, "lr": 0x710002A464, "inbytes":     4, "outbytes":     0},
  },
  '0x710002D8F0': { # , vtable size 3, possible vtables [0x710007D5F8 3, 0x710007B578 3, 0x7100078478 3]
      0:     {"vt":  0x20, "lr": 0x710002DAD4, "inbytes":  0x10, "outbytes":     0, "outinterfaces": ['0x710002E05C']},
      1:     {"vt":  0x28, "lr": 0x710002DCD8, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x710002E05C']},
      4:     {"vt":  0x30, "lr": 0x710002DED4, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x710002E05C']},
  },
  '0x710002E05C': { # , vtable size 4, possible vtables [0x710007B950 4, 0x710007DBC8 4]
      10:    {"vt":  0x20, "lr": 0x710002E280, "inbytes":     4, "outbytes":     0, "buffers": [33]},
      11:    {"vt":  0x28, "lr": 0x710002E44C, "inbytes":     4, "outbytes":     0, "buffers": [34]},
      12:    {"vt":  0x30, "lr": 0x710002E61C, "inbytes":     0, "outbytes":     0, "buffers": [34, 9]},
      13:    {"vt":  0x38, "lr": 0x710002E7D8, "inbytes":     8, "outbytes":     0},
  },
  '0x7100031520': { # , vtable size 1, possible vtables [0x710007BE10 1, 0x710007DA08 1, 0x7100078410 1, 0x71000783B8 1, 0x710007DB60 1, 0x710007BD48 1]
      0:     {"vt":  0x20, "lr": 0x71000316AC, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100031834']},
  },
  '0x7100031834': { # , vtable size 21, possible vtables []
      0:     {"vt":  0x20, "lr": 0x7100031B28, "inbytes":     0, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x7100031C78, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "lr": 0x7100031DCC, "inbytes":     1, "outbytes":     0},
      3:     {"vt":  0x38, "lr": 0x7100031F34, "inbytes":     0, "outbytes":     1},
      4:     {"vt":  0x40, "lr": 0x71000320AC, "inbytes":     0, "outbytes":     1},
      5:     {"vt":  0x48, "lr": 0x7100032224, "inbytes":     8, "outbytes":     0},
      6:     {"vt":  0x50, "lr": 0x710003238C, "inbytes":     0, "outbytes":     8},
      7:     {"vt":  0x58, "lr": 0x7100032504, "inbytes":     8, "outbytes":     0},
      8:     {"vt":  0x60, "lr": 0x710003266C, "inbytes":     0, "outbytes":     8},
      9:     {"vt":  0x68, "lr": 0x71000327E4, "inbytes":     8, "outbytes":     0},
      10:    {"vt":  0x70, "lr": 0x710003294C, "inbytes":     0, "outbytes":     8},
      11:    {"vt":  0x78, "lr": 0x7100032AC4, "inbytes":     8, "outbytes":     0},
      12:    {"vt":  0x80, "lr": 0x7100032C2C, "inbytes":     0, "outbytes":     8},
      13:    {"vt":  0x88, "lr": 0x7100032DA4, "inbytes":     4, "outbytes":     0},
      14:    {"vt":  0x90, "lr": 0x7100032F0C, "inbytes":     0, "outbytes":     4},
      15:    {"vt":  0x98, "lr": 0x7100033084, "inbytes":     0, "outbytes":     4},
      16:    {"vt":  0xA0, "lr": 0x71000331FC, "inbytes":     8, "outbytes":     0},
      17:    {"vt":  0xA8, "lr": 0x7100033364, "inbytes":     0, "outbytes":     8},
      18:    {"vt":  0xB0, "lr": 0x71000334DC, "inbytes":     4, "outbytes":     0},
      19:    {"vt":  0xB8, "lr": 0x7100033644, "inbytes":     0, "outbytes":     4},
      20:    {"vt":  0xC0, "lr": 0x71000337BC, "inbytes":     0, "outbytes":     1},
  },
  '0x7100039788': { # , vtable size 3, possible vtables [0x710007D5F8 3, 0x710007B578 3, 0x7100078478 3]
      0:     {"vt":  0x20, "lr": 0x710003996C, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100039EEC']},
      1:     {"vt":  0x28, "lr": 0x7100039B68, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100039EEC']},
      2:     {"vt":  0x30, "lr": 0x7100039D64, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100039EEC']},
  },
  '0x7100039EEC': { # , vtable size 6, possible vtables [0x710007D6D8 6, 0x71000785F8 6]
      0:     {"vt":  0x20, "lr": 0x710003A104, "inbytes":     8, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x710003A26C, "inbytes":     0, "outbytes":     8},
      4:     {"vt":  0x30, "lr": 0x710003A3E4, "inbytes":     1, "outbytes":     0},
      5:     {"vt":  0x38, "lr": 0x710003A550, "inbytes":     0, "outbytes":     1},
      6:     {"vt":  0x40, "lr": 0x710003A6C8, "inbytes":     8, "outbytes":     0},
      7:     {"vt":  0x48, "lr": 0x710003A84C, "inbytes":     0, "outbytes":     8},
  },
  '0x710003C1B0': { # , vtable size 1, possible vtables [0x710007BE10 1, 0x710007DA08 1, 0x7100078410 1, 0x71000783B8 1, 0x710007DB60 1, 0x710007BD48 1]
      0:     {"vt":  0x20, "lr": 0x710003C33C, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x710003C4C4']},
  },
  '0x710003C4C4': { # , vtable size 4, possible vtables [0x710007B950 4, 0x710007DBC8 4]
      0:     {"vt":  0x20, "lr": 0x710003C7F0, "inbytes":     1, "outbytes":     0, "buffers": [33]},
      1:     {"vt":  0x28, "lr": 0x710003C9AC, "inbytes":     1, "outbytes":     0, "buffers": [34]},
      2:     {"vt":  0x30, "lr": 0x710003CB64, "inbytes":  0x18, "outbytes":     0, "inhandles": [1]},
      3:     {"vt":  0x38, "lr": 0x710003C5DC, "inbytes":     0, "outbytes":     0},
  },
  'N2nn2sf22UnmanagedServiceObjectINS_4uart8IManagerENS2_6server11ManagerImplEEE': {
      0:     {"vt":  0x20, "func": 0x710003FB08, "lr": 0x710003FEB0, "inbytes":     4, "outbytes":     1},
      1:     {"vt":  0x28, "func": 0x710003FB24, "lr": 0x7100040044, "inbytes":     4, "outbytes":     1},
      2:     {"vt":  0x30, "func": 0x710003FB40, "lr": 0x71000401D8, "inbytes":     8, "outbytes":     1},
      3:     {"vt":  0x38, "func": 0x710003FB5C, "lr": 0x710004036C, "inbytes":     8, "outbytes":     1},
      4:     {"vt":  0x40, "func": 0x710003FB78, "lr": 0x7100040500, "inbytes":     8, "outbytes":     1},
      5:     {"vt":  0x48, "func": 0x710003FB94, "lr": 0x7100040694, "inbytes":     8, "outbytes":     1},
      6:     {"vt":  0x50, "func": 0x710003FBB0, "lr": 0x710004082C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100040FE8']},
      7:     {"vt":  0x58, "func": 0x710003FBE0, "lr": 0x7100040A04, "inbytes":     8, "outbytes":     1},
      8:     {"vt":  0x60, "func": 0x710003FBFC, "lr": 0x7100040B98, "inbytes":     8, "outbytes":     1},
      9:     {"vt":  0x68, "func": 0x710003FC18, "lr": 0x7100040D2C, "inbytes":     8, "outbytes":     1},
      10:    {"vt":  0x70, "func": 0x710003FC34, "lr": 0x7100040EC0, "inbytes":     8, "outbytes":     1},
  },
  'N2nn2sf6detail38ObjectImplFactoryWithStatefulAllocatorINS0_13InterfaceInfoINS_4uart12IPortSessionEE4_tABINS6_5_tO2NINS4_6server15PortSessionImplEE4typeES5_E4typeENS0_24StatefulAllocationPolicyINS0_16ExpHeapAllocatorEEEE6ObjectE': {
      0:     {"vt":  0x20, "func": 0x7100042C78, "lr": 0x7100041238, "inbytes":  0x28, "outbytes":     1, "inhandles": [1, 1]},
      1:     {"vt":  0x28, "func": 0x7100042CF4, "lr": 0x710004149C, "inbytes":  0x28, "outbytes":     1, "inhandles": [1, 1]},
      2:     {"vt":  0x30, "func": 0x7100042D70, "lr": 0x71000416E8, "inbytes":     0, "outbytes":     8},
      3:     {"vt":  0x38, "func": 0x7100042D9C, "lr": 0x7100041874, "inbytes":     0, "outbytes":     8, "buffers": [33]},
      4:     {"vt":  0x40, "func": 0x7100042DD8, "lr": 0x7100041A38, "inbytes":     0, "outbytes":     8},
      5:     {"vt":  0x48, "func": 0x7100042E04, "lr": 0x7100041BC4, "inbytes":     0, "outbytes":     8, "buffers": [34]},
      6:     {"vt":  0x50, "func": 0x7100042E3C, "lr": 0x7100041D94, "inbytes":  0x10, "outbytes":     1, "outhandles": [1]},
      7:     {"vt":  0x58, "func": 0x7100042F00, "lr": 0x7100041F78, "inbytes":     4, "outbytes":     1},
      8:     {"vt":  0x60, "func": 0x7100042F90, "lr": 0x7100042124, "inbytes":  0x28, "outbytes":     0, "inhandles": [1, 1]},
  },
},
        "#,
        );

        try_parse_ipc_file(
            r#"
'am': {
  '0x710000194C': { # ['nn::lm::ILogService', 'nn::am::service::IApplicationProxyService']
      0:     {"vt":  0x20, "func": 0x7100001644, "lr": 0x7100001AE4, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x7100001CA8'], "pid": True},
  },
  '0x7100001CA8': { # 2be25c4344a20646 '0(0;o0)1(0;o1)2(0;o2)3(0;o3)4(0;o4)1000(0;o8)10(0;o5)11(0;o6)20(0;o7)'
      0:     {"vt":  0x20, "func": 0x710007BB08, "lr": 0x71000100D4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100010B8C']},
      1:     {"vt":  0x28, "func": 0x710007BB40, "lr": 0x71000102B0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100016B1C']},
      2:     {"vt":  0x30, "func": 0x710007BB78, "lr": 0x710001048C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001BE24']},
      3:     {"vt":  0x38, "func": 0x710007BBB0, "lr": 0x7100010668, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001CBEC']},
      4:     {"vt":  0x40, "func": 0x710007BBE8, "lr": 0x7100010844, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001D4A4']},
      10:    {"vt":  0x50, "func": 0x710007BC58, "lr": 0x710000BFD8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000C3C8']},
      11:    {"vt":  0x58, "func": 0x710007BC90, "lr": 0x710000C1B4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000F3B8']},
      20:    {"vt":  0x60, "func": 0x710007BCC8, "lr": 0x7100001E70, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100001FDC']},
      1000:  {"vt":  0x48, "func": 0x710007BC20, "lr": 0x7100010A20, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100020318']},
  },
  '0x7100001FDC': { # 674f07663cf0c8e1 '1(1;o0)10(2)11(4)12(2)13(4)14(2;b21)15(2;b21)20(4)21(0)22(1)23(0)24(0)25(10)26(6)27(6)28(0)29(0)30(2)31(0)32(2)33(0)34(0;b5)35(0)36(0)37(0)40(0)50(0)60(1)65(0)66(2)67(1)68(0)70(0)71(0)72(0)80(0)90(1)100(4)101(5;b69)102(1)110(0;b6,5)111(4;b6,5)120(4)121(0)122(0)123(0)124(1)130(0)131(2)140(0)141(0;o0)150(0)151(0;o0)160(0)170(1)180(4)181(20)190(4)200(0)500(2)1000(2;o1)1001(0)'
      1:     {"vt":  0x20, "func": 0x71000A5B84, "lr": 0x7100002484, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      10:    {"vt":  0x28, "func": 0x71000A5BB4, "lr": 0x7100002680, "inbytes":     8, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      11:    {"vt":  0x30, "func": 0x71000A5C24, "lr": 0x7100002888, "inbytes":  0x10, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      12:    {"vt":  0x38, "func": 0x71000A5C94, "lr": 0x7100002A94, "inbytes":     8, "outbytes":     0},
      13:    {"vt":  0x40, "func": 0x71000A5CFC, "lr": 0x7100002BFC, "inbytes":  0x10, "outbytes":     0},
      14:    {"vt":  0x48, "func": 0x71000A5D64, "lr": 0x7100002D84, "inbytes":     8, "outbytes":     0, "buffers": [21], "ininterfaces": ['0x7100008730']},
      15:    {"vt":  0x50, "func": 0x71000A5DF4, "lr": 0x7100003004, "inbytes":     8, "outbytes":     0, "buffers": [21]},
      20:    {"vt":  0x58, "func": 0x71000A5E68, "lr": 0x71000031C0, "inbytes":  0x10, "outbytes":     8},
      21:    {"vt":  0x60, "func": 0x71000A5E98, "lr": 0x710000335C, "inbytes":     0, "outbytes":     8},
      22:    {"vt":  0x68, "func": 0x71000A5EC4, "lr": 0x71000034D4, "inbytes":     4, "outbytes":     0},
      23:    {"vt":  0x70, "func": 0x71000A5F40, "lr": 0x710000363C, "inbytes":     0, "outbytes":  0x10},
      24:    {"vt":  0x78, "func": 0x71000A5F70, "lr": 0x71000037B4, "inbytes":     0, "outbytes":     2},
      25:    {"vt":  0x80, "func": 0x71000A5F94, "lr": 0x7100003934, "inbytes":  0x28, "outbytes":     8},
      26:    {"vt":  0x88, "func": 0x71000A5FC4, "lr": 0x7100003AE0, "inbytes":  0x18, "outbytes":  0x10},
      27:    {"vt":  0x90, "func": 0x71000A5FF4, "lr": 0x7100003C90, "inbytes":  0x18, "outbytes":  0x10},
      28:    {"vt":  0x98, "func": 0x71000A6028, "lr": 0x7100003E30, "inbytes":     0, "outbytes":  0x10},
      29:    {"vt":  0xA0, "func": 0x71000A6048, "lr": 0x7100003FB0, "inbytes":     0, "outbytes":  0x10},
      30:    {"vt":  0xA8, "func": 0x71000A606C, "lr": 0x7100004130, "inbytes":     8, "outbytes":     0},
      31:    {"vt":  0xB0, "func": 0x71000A60E0, "lr": 0x7100004294, "inbytes":     0, "outbytes":     0},
      32:    {"vt":  0xB8, "func": 0x71000A6150, "lr": 0x71000043E8, "inbytes":     8, "outbytes":     0},
      33:    {"vt":  0xC0, "func": 0x71000A61C8, "lr": 0x710000454C, "inbytes":     0, "outbytes":     0},
      34:    {"vt":  0xC8, "func": 0x71000A6238, "lr": 0x71000046B4, "inbytes":     0, "outbytes":     1, "buffers": [5]},
      35:    {"vt":  0xD0, "func": 0x71000A6260, "lr": 0x7100004868, "inbytes":     0, "outbytes":  0x10},
      36:    {"vt":  0xD8, "func": 0x71000A6280, "lr": 0x71000049E8, "inbytes":     0, "outbytes":     1},
      37:    {"vt":  0xE0, "func": 0x71000A62A0, "lr": 0x7100004B68, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      40:    {"vt":  0xE8, "func": 0x71000A62C8, "lr": 0x7100004D0C, "inbytes":     0, "outbytes":     1},
      50:    {"vt":  0xF0, "func": 0x71000A6340, "lr": 0x7100004E84, "inbytes":     0, "outbytes":  0x10},
      60:    {"vt":  0xF8, "func": 0x71000A636C, "lr": 0x7100004FFC, "inbytes":     1, "outbytes":     0},
      65:    {"vt": 0x100, "func": 0x71000A63C8, "lr": 0x7100005168, "inbytes":     0, "outbytes":     1},
      66:    {"vt": 0x108, "func": 0x71000A63E4, "lr": 0x71000052EC, "inbytes":     8, "outbytes":     0, "inhandles": [1]},
      67:    {"vt": 0x110, "func": 0x71000A6438, "lr": 0x7100005498, "inbytes":     4, "outbytes":     0},
      68:    {"vt": 0x118, "func": 0x71000A6454, "lr": 0x71000055FC, "inbytes":     0, "outbytes":     0},
      70:    {"vt": 0x120, "func": 0x71000A6470, "lr": 0x710000574C, "inbytes":     0, "outbytes":     0},
      71:    {"vt": 0x128, "func": 0x71000A6494, "lr": 0x710000589C, "inbytes":     0, "outbytes":     0},
      72:    {"vt": 0x130, "func": 0x71000A64B8, "lr": 0x71000059EC, "inbytes":     0, "outbytes":     0},
      80:    {"vt": 0x138, "func": 0x71000A64DC, "lr": 0x7100005B3C, "inbytes":     0, "outbytes":     0},
      90:    {"vt": 0x140, "func": 0x71000A6524, "lr": 0x7100005C90, "inbytes":     1, "outbytes":     0},
      100:   {"vt": 0x148, "func": 0x71000A6590, "lr": 0x7100005E0C, "inbytes":  0x10, "outbytes":     0, "inhandles": [1]},
      101:   {"vt": 0x150, "func": 0x71000A6614, "lr": 0x7100005FE4, "inbytes":  0x14, "outbytes":     0, "buffers": [69]},
      102:   {"vt": 0x158, "func": 0x71000A6670, "lr": 0x71000061AC, "inbytes":     1, "outbytes":     0},
      110:   {"vt": 0x160, "func": 0x71000A668C, "lr": 0x7100006334, "inbytes":     0, "outbytes":     4, "buffers": [6, 5]},
      111:   {"vt": 0x168, "func": 0x71000A6744, "lr": 0x7100006524, "inbytes":  0x10, "outbytes":     4, "buffers": [6, 5]},
      120:   {"vt": 0x170, "func": 0x71000A6800, "lr": 0x7100006720, "inbytes":  0x10, "outbytes":     0},
      121:   {"vt": 0x178, "func": 0x71000A681C, "lr": 0x7100006888, "inbytes":     0, "outbytes":     0},
      122:   {"vt": 0x180, "func": 0x71000A68B4, "lr": 0x71000069E0, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      123:   {"vt": 0x188, "func": 0x71000A6900, "lr": 0x7100006BBC, "inbytes":     0, "outbytes":     4},
      124:   {"vt": 0x190, "func": 0x71000A6954, "lr": 0x7100006D34, "inbytes":     1, "outbytes":     0},
      130:   {"vt": 0x198, "func": 0x71000A69C0, "lr": 0x7100006EA8, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      131:   {"vt": 0x1A0, "func": 0x71000A6A10, "lr": 0x710000704C, "inbytes":     8, "outbytes":     0},
      140:   {"vt": 0x1A8, "func": 0x71000A6A78, "lr": 0x71000071BC, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      141:   {"vt": 0x1B0, "func": 0x71000A6AE8, "lr": 0x7100007364, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      150:   {"vt": 0x1B8, "func": 0x71000A6B8C, "lr": 0x7100007544, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      151:   {"vt": 0x1C0, "func": 0x71000A6BFC, "lr": 0x71000076EC, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      160:   {"vt": 0x1C8, "func": 0x71000A6CA0, "lr": 0x71000078CC, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      170:   {"vt": 0x1D0, "func": 0x71000A6CF0, "lr": 0x7100007A70, "inbytes":     1, "outbytes":     0},
      180:   {"vt": 0x1D8, "func": 0x71000A6D10, "lr": 0x7100007BE4, "inbytes":  0x10, "outbytes":  0x40, "pid": True},
      181:   {"vt": 0x1E0, "func": 0x71000A6D50, "lr": 0x7100007DD4, "inbytes":  0x50, "outbytes":     0, "pid": True},
      190:   {"vt": 0x1E8, "func": 0x71000A6D90, "lr": 0x7100007F88, "inbytes":  0x10, "outbytes":     0},
      200:   {"vt": 0x1F0, "func": 0x71000A6DB4, "lr": 0x71000080F0, "inbytes":     0, "outbytes":     4},
      500:   {"vt": 0x1F8, "func": 0x71000A6DE8, "lr": 0x7100008274, "inbytes":     8, "outbytes":     0, "outhandles": [1]},
      1000:  {"vt": 0x200, "func": 0x71000A6E04, "lr": 0x7100008444, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x710000972C']},
      1001:  {"vt": 0x208, "func": 0x71000A6E6C, "lr": 0x7100008648, "inbytes":     0, "outbytes":     0},
  },
  '0x7100008730': { # , vtable size 2, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x71000088E4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008C2C']},
      1:     {"vt":  0x28, "lr": 0x7100008AC0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000928C']},
  },
  '0x7100008C2C': { # , vtable size 3, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x7100008E08, "inbytes":     0, "outbytes":     8},
      10:    {"vt":  0x28, "lr": 0x7100008F94, "inbytes":     8, "outbytes":     0, "buffers": [33]},
      11:    {"vt":  0x30, "lr": 0x7100009150, "inbytes":     8, "outbytes":     0, "buffers": [34]},
  },
  '0x710000928C': { # , vtable size 2, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x710000943C, "inbytes":     0, "outbytes":     8},
      1:     {"vt":  0x28, "lr": 0x71000095C0, "inbytes":     0, "outbytes":     8, "outhandles": [1]},
  },
  '0x710000972C': { # , vtable size 2, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x71000098E0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100009BC4']},
      1:     {"vt":  0x28, "lr": 0x7100009AB8, "inbytes":     0, "outbytes":     8},
  },
  '0x7100009BC4': { # single hash match 'nn::grcsrv::IMovieMaker'
      2:     {"vt":  0x20, "func": 0x71000ADF78, "lr": 0x7100009E94, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000B6C8']},
      9:     {"vt":  0x28, "func": 0x71000ADFFC, "lr": 0x710000A06C, "inbytes":     8, "outbytes":     0},
      10:    {"vt":  0x30, "func": 0x71000AE078, "lr": 0x710000A1D4, "inbytes":     8, "outbytes":     4},
      11:    {"vt":  0x38, "func": 0x71000AE0C4, "lr": 0x710000A368, "inbytes":     8, "outbytes":     0},
      20:    {"vt":  0x40, "func": 0x71000AE120, "lr": 0x710000A4D0, "inbytes":     8, "outbytes":     0},
      21:    {"vt":  0x48, "func": 0x71000AE1D4, "lr": 0x710000A638, "inbytes":     8, "outbytes":     0},
      22:    {"vt":  0x50, "func": 0x71000AE24C, "lr": 0x710000A7A0, "inbytes":     8, "outbytes":     0},
      23:    {"vt":  0x58, "func": 0x71000AE304, "lr": 0x710000A91C, "inbytes":     8, "outbytes":     0, "buffers": [5]},
      24:    {"vt":  0x60, "func": 0x71000AE32C, "lr": 0x710000AAC4, "inbytes":  0x88, "outbytes":     0},
      25:    {"vt":  0x68, "func": 0x71000AE354, "lr": 0x710000AC60, "inbytes":  0x10, "outbytes":     0, "buffers": [5, 5]},
      26:    {"vt":  0x70, "func": 0x71000AE374, "lr": 0x710000AE48, "inbytes":  0x10, "outbytes":  0x20, "buffers": [5, 5]},
      30:    {"vt":  0x78, "func": 0x71000AE394, "lr": 0x710000B04C, "inbytes":     8, "outbytes":     0},
      41:    {"vt":  0x80, "func": 0x71000AE438, "lr": 0x710000B1C4, "inbytes":     8, "outbytes":     8, "buffers": [5]},
      50:    {"vt":  0x88, "func": 0x71000AE484, "lr": 0x710000B3A8, "inbytes":     8, "outbytes":     0, "outhandles": [1]},
      52:    {"vt":  0x90, "func": 0x71000AE4A4, "lr": 0x710000B574, "inbytes":     8, "outbytes":     0, "outhandles": [1]},
  },
  '0x710000B6C8': { # , vtable size 4, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4]
      0:     {"vt":  0x20, "lr": 0x710000B8F4, "inbytes":   0xC, "outbytes":     0, "buffers": [5, 6]},
      1:     {"vt":  0x28, "lr": 0x710000BAB8, "inbytes":   0xC, "outbytes":     0},
      2:     {"vt":  0x30, "lr": 0x710000BC30, "inbytes":     8, "outbytes":     0, "outhandles": [1]},
      3:     {"vt":  0x38, "lr": 0x710000BE10, "inbytes":   0xC, "outbytes":     0, "buffers": [33, 34]},
  },
  '0x710000C3C8': { # , vtable size 8, possible vtables [0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8]
      0:     {"vt":  0x20, "lr": 0x710000C608, "inbytes":     0, "outbytes":     4},
      11:    {"vt":  0x28, "lr": 0x710000C784, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000D30C']},
      21:    {"vt":  0x30, "lr": 0x710000C960, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      22:    {"vt":  0x38, "lr": 0x710000CB40, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      23:    {"vt":  0x40, "lr": 0x710000CD14, "inbytes":     0, "outbytes":     0},
      30:    {"vt":  0x48, "lr": 0x710000CE64, "inbytes":     0, "outbytes":     0},
      40:    {"vt":  0x50, "lr": 0x710000CFBC, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x710000D30C']},
      41:    {"vt":  0x58, "lr": 0x710000D19C, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x710000D30C']},
  },
  '0x710000D30C': { # a85851d05ebc720a '0(0)1(0)10(0)20(0)25(0)30(0)50(1)60(0)100(0)101(0;o0)102(0)103(0)104(0;o0)105(0)106(0)110(0)120(0)150(0)160(2)'
      0:     {"vt":  0x28, "func": 0x71000A953C, "lr": 0x710000EBC8, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      1:     {"vt":  0x30, "func": 0x71000A9558, "lr": 0x710000ED6C, "inbytes":     0, "outbytes":     1},
      10:    {"vt":  0x38, "func": 0x71000A9574, "lr": 0x710000EEE0, "inbytes":     0, "outbytes":     0},
      20:    {"vt":  0x40, "func": 0x71000A9590, "lr": 0x710000F030, "inbytes":     0, "outbytes":     0},
      25:    {"vt":  0x48, "func": 0x71000A95AC, "lr": 0x710000F180, "inbytes":     0, "outbytes":     0},
      30:    {"vt":  0x50, "func": 0x71000A95C8, "lr": 0x710000F2D0, "inbytes":     0, "outbytes":     0},
      50:    {"vt":  0x58, "func": 0x71000A9640, "lr": 0x710000D580, "inbytes":     1, "outbytes":     0},
      60:    {"vt":  0x60, "func": 0x71000A9654, "lr": 0x710000D6E8, "inbytes":     0, "outbytes":     0},
      100:   {"vt":  0x68, "func": 0x71000A9670, "lr": 0x710000D840, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      101:   {"vt":  0x70, "func": 0x71000A9738, "lr": 0x710000DA20, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      102:   {"vt":  0x78, "func": 0x71000A97B4, "lr": 0x710000DBFC, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      103:   {"vt":  0x80, "func": 0x71000A987C, "lr": 0x710000DDDC, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      104:   {"vt":  0x88, "func": 0x71000A9944, "lr": 0x710000DFBC, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      105:   {"vt":  0x90, "func": 0x71000A99C0, "lr": 0x710000E19C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      106:   {"vt":  0x98, "func": 0x71000A9A28, "lr": 0x710000E348, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      110:   {"vt":  0xA0, "func": 0x71000A9A90, "lr": 0x710000E4EC, "inbytes":     0, "outbytes":     1},
      120:   {"vt":  0xA8, "func": 0x71000A9ABC, "lr": 0x710000E664, "inbytes":     0, "outbytes":     8},
      150:   {"vt":  0xB0, "func": 0x71000A9AD4, "lr": 0x710000E7D8, "inbytes":     0, "outbytes":     0},
      160:   {"vt":  0xB8, "func": 0x71000A9B00, "lr": 0x710000E930, "inbytes":     8, "outbytes":     8, "pid": True},
  },
  '0x710000F3B8': { # , vtable size 6, possible vtables [0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6]
      0:     {"vt":  0x20, "lr": 0x710000F5D8, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x710000D30C']},
      1:     {"vt":  0x28, "lr": 0x710000F7C8, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "lr": 0x710000F91C, "inbytes":     0, "outbytes":     1},
      10:    {"vt":  0x38, "lr": 0x710000FA9C, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      11:    {"vt":  0x40, "lr": 0x710000FCA4, "inbytes":  0x10, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x7100008730']},
      12:    {"vt":  0x48, "lr": 0x710000FEC8, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x7100008730']},
  },
  '0x7100010B8C': { # 18e2f4ec067cc750 '0(0)1(0)2(0)3(0)4(0)5(0)6(0)7(0)8(0)9(0)10(0)11(0)12(0)13(0)14(0)20(0)30(0;o0)31(1;o0)32(1;o0)40(0)50(0)51(1)52(1)53(0)54(0)55(0)59(4)60(0)61(0)62(0)63(0)64(1)65(0;b5)66(1)67(0)68(0)80(1)90(1)91(0)100(1)110(0;o1)120(0;b6)200(0)300(0)400(0)401(0)500(0)501(2)502(0)503(0)900(0)'
      0:     {"vt":  0x20, "func": 0x71000A463C, "lr": 0x7100010FCC, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      1:     {"vt":  0x28, "func": 0x71000A4658, "lr": 0x7100011170, "inbytes":     0, "outbytes":     4},
      2:     {"vt":  0x30, "func": 0x71000A4674, "lr": 0x71000112E8, "inbytes":     0, "outbytes":     8},
      3:     {"vt":  0x38, "func": 0x71000A4690, "lr": 0x710001145C, "inbytes":     0, "outbytes":     0},
      4:     {"vt":  0x40, "func": 0x71000A46AC, "lr": 0x71000115AC, "inbytes":     0, "outbytes":     0},
      5:     {"vt":  0x48, "func": 0x71000A46C8, "lr": 0x7100011700, "inbytes":     0, "outbytes":     1},
      6:     {"vt":  0x50, "func": 0x71000A46E4, "lr": 0x7100011878, "inbytes":     0, "outbytes":     4},
      7:     {"vt":  0x58, "func": 0x71000A4700, "lr": 0x71000119F0, "inbytes":     0, "outbytes":     1},
      8:     {"vt":  0x60, "func": 0x71000A471C, "lr": 0x7100011B68, "inbytes":     0, "outbytes":     1},
      9:     {"vt":  0x68, "func": 0x71000A4738, "lr": 0x7100011CE0, "inbytes":     0, "outbytes":     1},
      10:    {"vt":  0x70, "func": 0x71000A4754, "lr": 0x7100011E54, "inbytes":     0, "outbytes":     0},
      11:    {"vt":  0x78, "func": 0x71000A4770, "lr": 0x7100011FA4, "inbytes":     0, "outbytes":     0},
      12:    {"vt":  0x80, "func": 0x71000A478C, "lr": 0x71000120F4, "inbytes":     0, "outbytes":     0},
      13:    {"vt":  0x88, "func": 0x71000A47A8, "lr": 0x7100012250, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      14:    {"vt":  0x90, "func": 0x71000A47C4, "lr": 0x71000123F4, "inbytes":     0, "outbytes":     8},
      20:    {"vt":  0x98, "func": 0x71000A47E0, "lr": 0x7100012570, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      30:    {"vt":  0xA0, "func": 0x71000A4834, "lr": 0x7100012750, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100015BA8']},
      31:    {"vt":  0xA8, "func": 0x71000A4864, "lr": 0x7100012930, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100015BA8']},
      32:    {"vt":  0xB0, "func": 0x71000A489C, "lr": 0x7100012B2C, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100015BA8']},
      40:    {"vt":  0xB8, "func": 0x71000A48D4, "lr": 0x7100012D20, "inbytes":     0, "outbytes":  0x10},
      50:    {"vt":  0xC0, "func": 0x71000A48F0, "lr": 0x7100012EA8, "inbytes":     0, "outbytes":     1},
      51:    {"vt":  0xC8, "func": 0x71000A490C, "lr": 0x7100013020, "inbytes":     1, "outbytes":     0},
      52:    {"vt":  0xD0, "func": 0x71000A492C, "lr": 0x710001318C, "inbytes":     1, "outbytes":     0},
      53:    {"vt":  0xD8, "func": 0x71000A494C, "lr": 0x71000132F4, "inbytes":     0, "outbytes":     0},
      54:    {"vt":  0xE0, "func": 0x71000A4968, "lr": 0x7100013444, "inbytes":     0, "outbytes":     0},
      55:    {"vt":  0xE8, "func": 0x71000A4984, "lr": 0x7100013598, "inbytes":     0, "outbytes":     1},
      59:    {"vt":  0xF0, "func": 0x71000A49A0, "lr": 0x7100013710, "inbytes":  0x10, "outbytes":     0},
      60:    {"vt":  0xF8, "func": 0x71000A49BC, "lr": 0x710001387C, "inbytes":     0, "outbytes":     8},
      61:    {"vt": 0x100, "func": 0x71000A49E0, "lr": 0x7100013A04, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      62:    {"vt": 0x108, "func": 0x71000A49FC, "lr": 0x7100013BA8, "inbytes":     0, "outbytes":     4},
      63:    {"vt": 0x110, "func": 0x71000A4A18, "lr": 0x7100013D28, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      64:    {"vt": 0x118, "func": 0x71000A4A34, "lr": 0x7100013ECC, "inbytes":     4, "outbytes":     0},
      65:    {"vt": 0x120, "func": 0x71000A4A50, "lr": 0x7100014048, "inbytes":     0, "outbytes":     8, "buffers": [5]},
      66:    {"vt": 0x128, "func": 0x71000A4A74, "lr": 0x71000141FC, "inbytes":     4, "outbytes":     0},
      67:    {"vt": 0x130, "func": 0x71000A4A90, "lr": 0x7100014360, "inbytes":     0, "outbytes":     0},
      68:    {"vt": 0x138, "func": 0x71000A4AAC, "lr": 0x71000144B4, "inbytes":     0, "outbytes":     4},
      80:    {"vt": 0x140, "func": 0x71000A4AC8, "lr": 0x710001462C, "inbytes":     4, "outbytes":     0},
      90:    {"vt": 0x148, "func": 0x71000A4AE4, "lr": 0x7100014794, "inbytes":     1, "outbytes":     0},
      91:    {"vt": 0x150, "func": 0x71000A4B04, "lr": 0x7100014900, "inbytes":     0, "outbytes":     4},
      100:   {"vt": 0x158, "func": 0x71000A4B20, "lr": 0x7100014A78, "inbytes":     1, "outbytes":     0},
      110:   {"vt": 0x160, "func": 0x71000A4B40, "lr": 0x7100014BE8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100016380']},
      120:   {"vt": 0x168, "func": 0x71000A4B70, "lr": 0x7100014DD4, "inbytes":     0, "outbytes":     4, "buffers": [6]},
      200:   {"vt": 0x170, "func": 0x71000A4B94, "lr": 0x7100014F8C, "inbytes":     0, "outbytes":     4},
      300:   {"vt": 0x178, "func": 0x71000A4BB0, "lr": 0x7100015104, "inbytes":     0, "outbytes":     1},
      400:   {"vt": 0x180, "func": 0x71000A4BCC, "lr": 0x7100015278, "inbytes":     0, "outbytes":     0},
      401:   {"vt": 0x188, "func": 0x71000A4BE8, "lr": 0x71000153C8, "inbytes":     0, "outbytes":     0},
      500:   {"vt": 0x190, "func": 0x71000A4C04, "lr": 0x7100015518, "inbytes":     0, "outbytes":     0},
      501:   {"vt": 0x198, "func": 0x71000A4C20, "lr": 0x710001566C, "inbytes":     8, "outbytes":     0},
      502:   {"vt": 0x1A0, "func": 0x71000A4C3C, "lr": 0x71000157D4, "inbytes":     0, "outbytes":     1},
      503:   {"vt": 0x1A8, "func": 0x71000A4C58, "lr": 0x710001594C, "inbytes":     0, "outbytes":     1},
      900:   {"vt": 0x1B0, "func": 0x71000A4C74, "lr": 0x7100015AC0, "inbytes":     0, "outbytes":     0},
  },
  '0x7100015BA8': { # , vtable size 4, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4]
      1:     {"vt":  0x20, "lr": 0x7100015ECC, "inbytes":     1, "outbytes":     1, "outhandles": [1]},
      2:     {"vt":  0x28, "lr": 0x7100015CEC, "inbytes":     0, "outbytes":     0},
      3:     {"vt":  0x30, "lr": 0x71000160D0, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      4:     {"vt":  0x38, "lr": 0x7100016274, "inbytes":     0, "outbytes":     1},
  },
  '0x7100016380': { # , vtable size 4, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4]
      100:   {"vt":  0x20, "lr": 0x710001669C, "inbytes":     0, "outbytes":     8},
      101:   {"vt":  0x28, "lr": 0x7100016828, "inbytes":     0, "outbytes":     8, "buffers": [6]},
      102:   {"vt":  0x30, "lr": 0x71000169E4, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      103:   {"vt":  0x38, "lr": 0x710001649C, "inbytes":     0, "outbytes":     0},
  },
  '0x7100016B1C': { # 59d3d3ed74f32c9f '0(0)1(0)2(0)3(0)4(0)9(0)10(1)11(1)12(1)13(1)14(1)15(4)16(1)17(1)18(1)19(1)20(1)21(0)40(0)41(0)42(0)43(0)44(0)45(1)46(1)50(1)51(0)60(4)61(1)62(1)63(0)64(1)65(0)66(0)67(0)68(1)69(0)70(1;b5)71(0)72(1)80(1)90(0)91(0)100(1)110(0;b33)120(1)130(1)1000(0;o0)'
      0:     {"vt":  0x20, "func": 0x71000A4D10, "lr": 0x7100016F08, "inbytes":     0, "outbytes":     0},
      1:     {"vt":  0x28, "func": 0x71000A4D34, "lr": 0x7100017058, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "func": 0x71000A4D50, "lr": 0x71000171A8, "inbytes":     0, "outbytes":     0},
      3:     {"vt":  0x38, "func": 0x71000A4D6C, "lr": 0x71000172F8, "inbytes":     0, "outbytes":     0},
      4:     {"vt":  0x40, "func": 0x71000A4D88, "lr": 0x7100017448, "inbytes":     0, "outbytes":     0},
      9:     {"vt":  0x48, "func": 0x71000A4DA4, "lr": 0x71000175A4, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      10:    {"vt":  0x50, "func": 0x71000A4DC0, "lr": 0x7100017748, "inbytes":     4, "outbytes":     0},
      11:    {"vt":  0x58, "func": 0x71000A4DDC, "lr": 0x71000178B0, "inbytes":     1, "outbytes":     0},
      12:    {"vt":  0x60, "func": 0x71000A4DFC, "lr": 0x7100017A1C, "inbytes":     1, "outbytes":     0},
      13:    {"vt":  0x68, "func": 0x71000A4E1C, "lr": 0x7100017B88, "inbytes":     3, "outbytes":     0},
      14:    {"vt":  0x70, "func": 0x71000A4E44, "lr": 0x7100017D04, "inbytes":     1, "outbytes":     0},
      15:    {"vt":  0x78, "func": 0x71000A4E64, "lr": 0x7100017E70, "inbytes":  0x10, "outbytes":     0},
      16:    {"vt":  0x80, "func": 0x71000A4E88, "lr": 0x7100017FE0, "inbytes":     1, "outbytes":     0},
      17:    {"vt":  0x88, "func": 0x71000A4EA8, "lr": 0x710001814C, "inbytes":     1, "outbytes":     0},
      18:    {"vt":  0x90, "func": 0x71000A4EC8, "lr": 0x71000182B8, "inbytes":     1, "outbytes":     0},
      19:    {"vt":  0x98, "func": 0x71000A4EE8, "lr": 0x7100018424, "inbytes":     4, "outbytes":     0},
      20:    {"vt":  0xA0, "func": 0x71000A4F04, "lr": 0x710001858C, "inbytes":     4, "outbytes":     0},
      21:    {"vt":  0xA8, "func": 0x71000A4F20, "lr": 0x71000186F4, "inbytes":     0, "outbytes":     8},
      40:    {"vt":  0xB0, "func": 0x71000A4F3C, "lr": 0x710001886C, "inbytes":     0, "outbytes":     8},
      41:    {"vt":  0xB8, "func": 0x71000A4F58, "lr": 0x71000189E0, "inbytes":     0, "outbytes":     0},
      42:    {"vt":  0xC0, "func": 0x71000A4F74, "lr": 0x7100018B34, "inbytes":     0, "outbytes":  0x10},
      43:    {"vt":  0xC8, "func": 0x71000A4F90, "lr": 0x7100018CB4, "inbytes":     0, "outbytes":     8},
      44:    {"vt":  0xD0, "func": 0x71000A4FAC, "lr": 0x7100018E2C, "inbytes":     0, "outbytes":  0x10},
      45:    {"vt":  0xD8, "func": 0x71000A4FC8, "lr": 0x7100018FAC, "inbytes":     4, "outbytes":     0},
      46:    {"vt":  0xE0, "func": 0x71000A4FE4, "lr": 0x7100019114, "inbytes":     1, "outbytes":     0},
      50:    {"vt":  0xE8, "func": 0x71000A5004, "lr": 0x7100019280, "inbytes":     1, "outbytes":     0},
      51:    {"vt":  0xF0, "func": 0x71000A5024, "lr": 0x71000193E8, "inbytes":     0, "outbytes":     0},
      60:    {"vt":  0xF8, "func": 0x71000A5040, "lr": 0x710001953C, "inbytes":  0x10, "outbytes":     0},
      61:    {"vt": 0x100, "func": 0x71000A505C, "lr": 0x71000196A8, "inbytes":     1, "outbytes":     0},
      62:    {"vt": 0x108, "func": 0x71000A5084, "lr": 0x7100019814, "inbytes":     4, "outbytes":     0},
      63:    {"vt": 0x110, "func": 0x71000A50A0, "lr": 0x710001997C, "inbytes":     0, "outbytes":     4},
      64:    {"vt": 0x118, "func": 0x71000A50BC, "lr": 0x7100019AF4, "inbytes":     4, "outbytes":     0},
      65:    {"vt": 0x120, "func": 0x71000A50D8, "lr": 0x7100019C58, "inbytes":     0, "outbytes":     0},
      66:    {"vt": 0x128, "func": 0x71000A50F4, "lr": 0x7100019DAC, "inbytes":     0, "outbytes":     4},
      67:    {"vt": 0x130, "func": 0x71000A5110, "lr": 0x7100019F24, "inbytes":     0, "outbytes":     1},
      68:    {"vt": 0x138, "func": 0x71000A512C, "lr": 0x710001A09C, "inbytes":     1, "outbytes":     0},
      69:    {"vt": 0x140, "func": 0x71000A514C, "lr": 0x710001A208, "inbytes":     0, "outbytes":     1},
      70:    {"vt": 0x148, "func": 0x71000A5168, "lr": 0x710001A394, "inbytes":     4, "outbytes":     0, "buffers": [5]},
      71:    {"vt": 0x150, "func": 0x71000A5184, "lr": 0x710001A53C, "inbytes":     0, "outbytes":     8},
      72:    {"vt": 0x158, "func": 0x71000A51A0, "lr": 0x710001A6BC, "inbytes":     4, "outbytes":     0},
      80:    {"vt": 0x160, "func": 0x71000A51BC, "lr": 0x710001A824, "inbytes":     4, "outbytes":     0},
      90:    {"vt": 0x168, "func": 0x71000A51D8, "lr": 0x710001A98C, "inbytes":     0, "outbytes":     8},
      91:    {"vt": 0x170, "func": 0x71000A51F4, "lr": 0x710001AB0C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      100:   {"vt": 0x178, "func": 0x71000A5210, "lr": 0x710001ACB0, "inbytes":     1, "outbytes":     0},
      110:   {"vt": 0x180, "func": 0x71000A5230, "lr": 0x710001AE2C, "inbytes":     0, "outbytes":     0, "buffers": [33]},
      120:   {"vt": 0x188, "func": 0x71000A5254, "lr": 0x710001AFBC, "inbytes":     4, "outbytes":     0},
      130:   {"vt": 0x190, "func": 0x71000A5270, "lr": 0x710001B124, "inbytes":     1, "outbytes":     0},
      1000:  {"vt": 0x198, "func": 0x71000A5290, "lr": 0x710001B294, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001B400']},
  },
  '0x710001B400': { # , vtable size 5, possible vtables [0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5]
      0:     {"vt":  0x20, "lr": 0x710001B5FC, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      1:     {"vt":  0x28, "lr": 0x710001B7DC, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      2:     {"vt":  0x30, "lr": 0x710001B9BC, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      3:     {"vt":  0x38, "lr": 0x710001BB9C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      4:     {"vt":  0x40, "lr": 0x710001BD3C, "inbytes":     0, "outbytes":     0},
  },
  '0x710001BE24': { # , vtable size 8, possible vtables [0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8]
      0:     {"vt":  0x20, "lr": 0x710001C06C, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x710001CBA8']},
      1:     {"vt":  0x28, "lr": 0x710001C260, "inbytes":     0, "outbytes":     8},
      2:     {"vt":  0x30, "lr": 0x710001C3D8, "inbytes":     0, "outbytes":     8},
      10:    {"vt":  0x38, "lr": 0x710001C54C, "inbytes":     0, "outbytes":     0},
      11:    {"vt":  0x40, "lr": 0x710001C69C, "inbytes":     0, "outbytes":     0},
      12:    {"vt":  0x48, "lr": 0x710001C7EC, "inbytes":     0, "outbytes":     0},
      20:    {"vt":  0x50, "lr": 0x710001C940, "inbytes":     1, "outbytes":     0},
      21:    {"vt":  0x58, "lr": 0x710001CAAC, "inbytes":     8, "outbytes":     0},
  },
  '0x710001CBA8': { # , vtable size 0, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
  },
  '0x710001CBEC': { # , vtable size 5, possible vtables [0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5]
      0:     {"vt":  0x20, "lr": 0x710001CDE4, "inbytes":     8, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x710001CF4C, "inbytes":     0, "outbytes":     4},
      2:     {"vt":  0x30, "lr": 0x710001D0C4, "inbytes":     0, "outbytes":     4},
      3:     {"vt":  0x38, "lr": 0x710001D23C, "inbytes":  0x10, "outbytes":     0},
      4:     {"vt":  0x40, "lr": 0x710001D3A8, "inbytes":     4, "outbytes":     0},
  },
  '0x710001D4A4': { # single hash match 'nn::am::service::IDisplayController'
      0:     {"vt":  0x20, "func": 0x71000A544C, "lr": 0x710001D884, "inbytes":     0, "outbytes":     0, "buffers": [6]},
      1:     {"vt":  0x28, "func": 0x71000A5470, "lr": 0x710001DA10, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "func": 0x71000A548C, "lr": 0x710001DB74, "inbytes":     0, "outbytes":     0, "buffers": [6]},
      3:     {"vt":  0x38, "func": 0x71000A54B0, "lr": 0x710001DD14, "inbytes":     0, "outbytes":     0, "buffers": [6]},
      4:     {"vt":  0x40, "func": 0x71000A54D4, "lr": 0x710001DEA0, "inbytes":     0, "outbytes":     0},
      5:     {"vt":  0x48, "func": 0x71000A54F0, "lr": 0x710001E008, "inbytes":     0, "outbytes":     1, "buffers": [6]},
      6:     {"vt":  0x50, "func": 0x71000A5514, "lr": 0x710001E1D0, "inbytes":     0, "outbytes":     1, "buffers": [6]},
      7:     {"vt":  0x58, "func": 0x71000A5538, "lr": 0x710001E398, "inbytes":     0, "outbytes":     1, "buffers": [6]},
      8:     {"vt":  0x60, "func": 0x71000A555C, "lr": 0x710001E54C, "inbytes":     8, "outbytes":     0},
      9:     {"vt":  0x68, "func": 0x71000A557C, "lr": 0x710001E6BC, "inbytes":     8, "outbytes":     0},
      10:    {"vt":  0x70, "func": 0x71000A5598, "lr": 0x710001E82C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      11:    {"vt":  0x78, "func": 0x71000A55B4, "lr": 0x710001E9CC, "inbytes":     0, "outbytes":     0},
      12:    {"vt":  0x80, "func": 0x71000A55D0, "lr": 0x710001EB28, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      13:    {"vt":  0x88, "func": 0x71000A55EC, "lr": 0x710001ECC8, "inbytes":     0, "outbytes":     0},
      14:    {"vt":  0x90, "func": 0x71000A5608, "lr": 0x710001EE24, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      15:    {"vt":  0x98, "func": 0x71000A5624, "lr": 0x710001EFC4, "inbytes":     0, "outbytes":     0},
      16:    {"vt":  0xA0, "func": 0x71000A5640, "lr": 0x710001F124, "inbytes":     0, "outbytes":     1, "outhandles": [1]},
      17:    {"vt":  0xA8, "func": 0x71000A565C, "lr": 0x710001F308, "inbytes":     0, "outbytes":     1, "outhandles": [1]},
      18:    {"vt":  0xB0, "func": 0x71000A5678, "lr": 0x710001F4EC, "inbytes":     0, "outbytes":     1, "outhandles": [1]},
      20:    {"vt":  0xB8, "func": 0x71000A5694, "lr": 0x710001F6C4, "inbytes":   0xC, "outbytes":     0},
      21:    {"vt":  0xC0, "func": 0x71000A56B4, "lr": 0x710001F834, "inbytes":     4, "outbytes":     0},
      22:    {"vt":  0xC8, "func": 0x71000A56D0, "lr": 0x710001F99C, "inbytes":     0, "outbytes":     8},
      23:    {"vt":  0xD0, "func": 0x71000A56EC, "lr": 0x710001FB18, "inbytes":     0, "outbytes":     0},
      24:    {"vt":  0xD8, "func": 0x71000A5708, "lr": 0x710001FC6C, "inbytes":     0, "outbytes":     8},
      25:    {"vt":  0xE0, "func": 0x71000A5724, "lr": 0x710001FDE8, "inbytes":     0, "outbytes":     0},
      26:    {"vt":  0xE8, "func": 0x71000A5740, "lr": 0x710001FF3C, "inbytes":     0, "outbytes":     8},
      27:    {"vt":  0xF0, "func": 0x71000A575C, "lr": 0x71000200B8, "inbytes":     0, "outbytes":     0},
      28:    {"vt":  0xF8, "func": 0x71000A5778, "lr": 0x710002020C, "inbytes":     8, "outbytes":     0},
  },
  '0x7100020318': { # 20f7499d916dd994 '0(1)10(1)20(0)30(4;b5,5)31(18;b5,5)40(0)100(1)101(0)110(1)111(1;o0)120(2)121(0)122(0)130(2)131(0)132(0)140(0)900(0)'
      0:     {"vt":  0x20, "func": 0x710007C7A0, "lr": 0x7100020628, "inbytes":     4, "outbytes":     0},
      10:    {"vt":  0x28, "func": 0x710007C7C0, "lr": 0x7100020790, "inbytes":     4, "outbytes":     0},
      20:    {"vt":  0x30, "func": 0x710007C7DC, "lr": 0x71000208F4, "inbytes":     0, "outbytes":     0},
      30:    {"vt":  0x38, "func": 0x710007C7F8, "lr": 0x7100020A68, "inbytes":  0x10, "outbytes":     0, "buffers": [5, 5]},
      31:    {"vt":  0x40, "func": 0x710007C828, "lr": 0x7100020C48, "inbytes":  0x48, "outbytes":     0, "buffers": [5, 5]},
      40:    {"vt":  0x48, "func": 0x710007C858, "lr": 0x7100020E2C, "inbytes":     0, "outbytes":  0x20},
      100:   {"vt":  0x50, "func": 0x710007C89C, "lr": 0x7100020FB0, "inbytes":     4, "outbytes":     0},
      101:   {"vt":  0x58, "func": 0x710007C8B8, "lr": 0x7100021114, "inbytes":     0, "outbytes":     0},
      110:   {"vt":  0x60, "func": 0x710007C8D4, "lr": 0x7100021270, "inbytes":     4, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      111:   {"vt":  0x68, "func": 0x710007C920, "lr": 0x7100021470, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      120:   {"vt":  0x70, "func": 0x710007C950, "lr": 0x710002166C, "inbytes":     8, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      121:   {"vt":  0x78, "func": 0x710007C99C, "lr": 0x7100021860, "inbytes":     0, "outbytes":     0},
      122:   {"vt":  0x80, "func": 0x710007C9B8, "lr": 0x71000219B8, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      130:   {"vt":  0x88, "func": 0x710007CA04, "lr": 0x7100021B9C, "inbytes":     8, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      131:   {"vt":  0x90, "func": 0x710007CA50, "lr": 0x7100021D90, "inbytes":     0, "outbytes":     0},
      132:   {"vt":  0x98, "func": 0x710007CA6C, "lr": 0x7100021EE8, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      140:   {"vt":  0xA0, "func": 0x710007CAB8, "lr": 0x71000220C0, "inbytes":     0, "outbytes":     0},
      900:   {"vt":  0xA8, "func": 0x710007CAD4, "lr": 0x710002221C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
  },
  '0x7100022354': { # , vtable size 8, possible vtables [0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8]
      100:   {"vt":  0x20, "lr": 0x71000225DC, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x7100023670'], "pid": True},
      200:   {"vt":  0x28, "lr": 0x7100022820, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x710002BC90'], "pid": True},
      201:   {"vt":  0x30, "lr": 0x7100022A3C, "inbytes":     8, "outbytes":     0, "buffers": [21], "inhandles": [1], "outinterfaces": ['0x710002BC90'], "pid": True},
      300:   {"vt":  0x38, "lr": 0x7100022CB0, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x7100030668'], "pid": True},
      350:   {"vt":  0x40, "lr": 0x7100022EF4, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x7100001CA8'], "pid": True},
      400:   {"vt":  0x48, "lr": 0x7100023130, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x710000F3B8'], "pid": True},
      410:   {"vt":  0x50, "lr": 0x7100023328, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x71000322D8']},
      1000:  {"vt":  0x58, "lr": 0x7100023504, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100020318']},
  },
  '0x7100023670': { # faec0e73ade652c1 '0(0;o0)1(0;o1)2(0;o2)3(0;o3)4(0;o4)1000(0;o11)10(0;o5)11(0;o6)20(0;o7)21(0;o8)22(0;o9)23(0;o10)'
      0:     {"vt":  0x20, "func": 0x710007BD88, "lr": 0x71000100D4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100010B8C']},
      1:     {"vt":  0x28, "func": 0x710007BDC0, "lr": 0x71000102B0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100016B1C']},
      2:     {"vt":  0x30, "func": 0x710007BDF8, "lr": 0x710001048C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001BE24']},
      3:     {"vt":  0x38, "func": 0x710007BE30, "lr": 0x7100010668, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001CBEC']},
      4:     {"vt":  0x40, "func": 0x710007BE68, "lr": 0x7100010844, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001D4A4']},
      10:    {"vt":  0x50, "func": 0x710007BED8, "lr": 0x710000BFD8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000C3C8']},
      11:    {"vt":  0x58, "func": 0x710007BF10, "lr": 0x710000C1B4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000F3B8']},
      20:    {"vt":  0x60, "func": 0x710007BF48, "lr": 0x7100023894, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100023F94']},
      21:    {"vt":  0x68, "func": 0x710007BF80, "lr": 0x7100023A70, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100025620']},
      22:    {"vt":  0x70, "func": 0x710007BFB8, "lr": 0x7100023C4C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x71000274FC']},
      23:    {"vt":  0x78, "func": 0x710007BFF0, "lr": 0x7100023E28, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710002A164']},
      1000:  {"vt":  0x48, "func": 0x710007BEA0, "lr": 0x7100010A20, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100020318']},
  },
  '0x7100023F94': { # , vtable size 13, possible vtables [0x71001DD488 13, 0x71001DD3F0 13]
      10:    {"vt":  0x20, "lr": 0x7100024244, "inbytes":     0, "outbytes":     0},
      11:    {"vt":  0x28, "lr": 0x7100024394, "inbytes":     0, "outbytes":     0},
      12:    {"vt":  0x30, "lr": 0x71000244E4, "inbytes":     0, "outbytes":     0},
      20:    {"vt":  0x38, "lr": 0x710002463C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      21:    {"vt":  0x40, "lr": 0x710002481C, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      30:    {"vt":  0x48, "lr": 0x71000249C4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100015BA8']},
      31:    {"vt":  0x50, "lr": 0x7100024BA4, "inbytes":     4, "outbytes":     0, "outinterfaces": ['0x7100015BA8']},
      40:    {"vt":  0x58, "lr": 0x7100024D98, "inbytes":     0, "outbytes":     1},
      41:    {"vt":  0x60, "lr": 0x7100024F10, "inbytes":     0, "outbytes":     1},
      100:   {"vt":  0x68, "lr": 0x710002509C, "inbytes":     0, "outbytes":  0x10, "buffers": [6]},
      110:   {"vt":  0x70, "lr": 0x710002525C, "inbytes":     0, "outbytes":     1},
      200:   {"vt":  0x78, "lr": 0x71000253D0, "inbytes":     0, "outbytes":     0},
      1000:  {"vt":  0x80, "lr": 0x7100025524, "inbytes":     4, "outbytes":     0},
  },
  '0x7100025620': { # , vtable size 13, possible vtables [0x71001DD488 13, 0x71001DD3F0 13]
      0:     {"vt":  0x20, "lr": 0x71000258C0, "inbytes":     0, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x7100025A10, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "lr": 0x7100025B64, "inbytes":     1, "outbytes":     0},
      3:     {"vt":  0x38, "lr": 0x7100025CCC, "inbytes":     0, "outbytes":     0},
      4:     {"vt":  0x40, "lr": 0x7100025E1C, "inbytes":     0, "outbytes":     0},
      9:     {"vt":  0x48, "lr": 0x7100025F70, "inbytes":     0, "outbytes":     1},
      10:    {"vt":  0x50, "lr": 0x71000260E4, "inbytes":     0, "outbytes":     0},
      11:    {"vt":  0x58, "lr": 0x7100026234, "inbytes":     0, "outbytes":     0},
      12:    {"vt":  0x60, "lr": 0x7100026388, "inbytes":     8, "outbytes":     0},
      13:    {"vt":  0x68, "lr": 0x71000264EC, "inbytes":     0, "outbytes":     0},
      14:    {"vt":  0x70, "lr": 0x7100026640, "inbytes":     0, "outbytes":     1},
      15:    {"vt":  0x78, "lr": 0x71000267C0, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      30:    {"vt":  0x80, "lr": 0x7100026968, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100026AD4']},
  },
  '0x7100026AD4': { # , vtable size 6, possible vtables [0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6]
      0:     {"vt":  0x20, "lr": 0x7100026CD8, "inbytes":     0, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x7100026E28, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "lr": 0x7100026F7C, "inbytes":     0, "outbytes":  0x10},
      3:     {"vt":  0x38, "lr": 0x71000270FC, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      4:     {"vt":  0x40, "lr": 0x71000272A0, "inbytes":     0, "outbytes":  0x10},
      5:     {"vt":  0x48, "lr": 0x7100027414, "inbytes":     0, "outbytes":     0},
  },
  '0x71000274FC': { # , vtable size 4, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4]
      0:     {"vt":  0x20, "lr": 0x7100027718, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x7100027E54']},
      1:     {"vt":  0x28, "lr": 0x7100027910, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100027E54']},
      10:    {"vt":  0x30, "lr": 0x7100027AF0, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x7100027E54']},
      100:   {"vt":  0x38, "lr": 0x7100027CE8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100027E54']},
  },
  '0x7100027E54': { # 7120a1f0ff1fafe1 '0(0)1(0)10(0)20(0)25(0)30(0)101(0)110(0)111(0)112(0;o0)120(0)121(1)122(0;b6)123(0;b6)124(0)130(1;b5)131(0)132(0)140(0;b6)150(0)160(0;b21)170(2)180(0)190(0)200(0)201(0)'
      0:     {"vt":  0x28, "func": 0x71000A24EC, "lr": 0x710000EBC8, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      1:     {"vt":  0x30, "func": 0x71000A2508, "lr": 0x710000ED6C, "inbytes":     0, "outbytes":     1},
      10:    {"vt":  0x38, "func": 0x71000A2524, "lr": 0x710000EEE0, "inbytes":     0, "outbytes":     0},
      20:    {"vt":  0x40, "func": 0x71000A2540, "lr": 0x710000F030, "inbytes":     0, "outbytes":     0},
      25:    {"vt":  0x48, "func": 0x71000A255C, "lr": 0x710000F180, "inbytes":     0, "outbytes":     0},
      30:    {"vt":  0x50, "func": 0x71000A2578, "lr": 0x710000F2D0, "inbytes":     0, "outbytes":     0},
      101:   {"vt":  0x58, "func": 0x71000A25C4, "lr": 0x7100028134, "inbytes":     0, "outbytes":     0},
      110:   {"vt":  0x60, "func": 0x71000A25E0, "lr": 0x7100028284, "inbytes":     0, "outbytes":     0},
      111:   {"vt":  0x68, "func": 0x71000A2648, "lr": 0x71000283D8, "inbytes":     0, "outbytes":     1},
      112:   {"vt":  0x70, "func": 0x71000A2678, "lr": 0x7100028554, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710002A068']},
      120:   {"vt":  0x78, "func": 0x71000A26A8, "lr": 0x710002872C, "inbytes":     0, "outbytes":     8},
      121:   {"vt":  0x80, "func": 0x71000A26D4, "lr": 0x71000288AC, "inbytes":     4, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      122:   {"vt":  0x88, "func": 0x71000A2730, "lr": 0x7100028AB4, "inbytes":     0, "outbytes":     0, "buffers": [6]},
      123:   {"vt":  0x90, "func": 0x71000A27C0, "lr": 0x7100028C54, "inbytes":     0, "outbytes":     0, "buffers": [6]},
      124:   {"vt":  0x98, "func": 0x71000A2864, "lr": 0x7100028DE4, "inbytes":     0, "outbytes":  0x10},
      130:   {"vt":  0xA0, "func": 0x71000A287C, "lr": 0x7100028F70, "inbytes":     1, "outbytes":     0, "buffers": [5]},
      131:   {"vt":  0xA8, "func": 0x71000A289C, "lr": 0x710002911C, "inbytes":     0, "outbytes":     1},
      132:   {"vt":  0xB0, "func": 0x71000A28B8, "lr": 0x7100029294, "inbytes":     0, "outbytes":     8},
      140:   {"vt":  0xB8, "func": 0x71000A28D4, "lr": 0x7100029420, "inbytes":     0, "outbytes":     4, "buffers": [6]},
      150:   {"vt":  0xC0, "func": 0x71000A2958, "lr": 0x71000295D4, "inbytes":     0, "outbytes":     0},
      160:   {"vt":  0xC8, "func": 0x71000A29A0, "lr": 0x710002973C, "inbytes":     0, "outbytes":     0, "buffers": [21]},
      170:   {"vt":  0xD0, "func": 0x71000A29CC, "lr": 0x71000298E0, "inbytes":     8, "outbytes":     1},
      180:   {"vt":  0xD8, "func": 0x71000A29E8, "lr": 0x7100029A78, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      190:   {"vt":  0xE0, "func": 0x71000A2A34, "lr": 0x7100029C58, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      200:   {"vt":  0xE8, "func": 0x71000A2A80, "lr": 0x7100029E30, "inbytes":     0, "outbytes":     0},
      201:   {"vt":  0xF0, "func": 0x71000A2B20, "lr": 0x7100029F80, "inbytes":     0, "outbytes":     0},
  },
  '0x710002A068': { # e8228d2b01a6f6df '0(0)1(0)10(0)20(0)25(0)30(0)'
      0:     {"vt":  0x28, "func": 0x71000A2D6C, "lr": 0x710000EBC8, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      1:     {"vt":  0x30, "func": 0x71000A2D88, "lr": 0x710000ED6C, "inbytes":     0, "outbytes":     1},
      10:    {"vt":  0x38, "func": 0x71000A2DA4, "lr": 0x710000EEE0, "inbytes":     0, "outbytes":     0},
      20:    {"vt":  0x40, "func": 0x71000A2DC0, "lr": 0x710000F030, "inbytes":     0, "outbytes":     0},
      25:    {"vt":  0x48, "func": 0x71000A2DDC, "lr": 0x710000F180, "inbytes":     0, "outbytes":     0},
      30:    {"vt":  0x50, "func": 0x71000A2DF8, "lr": 0x710000F2D0, "inbytes":     0, "outbytes":     0},
  },
  '0x710002A164': { # , vtable size 16, possible vtables [0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x710002A444, "inbytes":     4, "outbytes":     0},
      10:    {"vt":  0x28, "lr": 0x710002A5BC, "inbytes":     8, "outbytes":     8, "buffers": [34]},
      11:    {"vt":  0x30, "lr": 0x710002A7A8, "inbytes":     8, "outbytes":     0, "buffers": [33]},
      20:    {"vt":  0x38, "lr": 0x710002A954, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      21:    {"vt":  0x40, "lr": 0x710002AB34, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      40:    {"vt":  0x48, "lr": 0x710002AD0C, "inbytes":     0, "outbytes":     8},
      42:    {"vt":  0x50, "lr": 0x710002AE8C, "inbytes":  0x10, "outbytes":     0},
      50:    {"vt":  0x58, "lr": 0x710002AFF8, "inbytes":     1, "outbytes":     0},
      51:    {"vt":  0x60, "lr": 0x710002B164, "inbytes":     0, "outbytes":     1},
      52:    {"vt":  0x68, "lr": 0x710002B2DC, "inbytes":     0, "outbytes":     1},
      60:    {"vt":  0x70, "lr": 0x710002B454, "inbytes":     0, "outbytes":     1},
      61:    {"vt":  0x78, "lr": 0x710002B5CC, "inbytes":     0, "outbytes":     1},
      62:    {"vt":  0x80, "lr": 0x710002B744, "inbytes":     0, "outbytes":     1},
      70:    {"vt":  0x88, "lr": 0x710002B8BC, "inbytes":     4, "outbytes":     0},
      80:    {"vt":  0x90, "lr": 0x710002BA24, "inbytes":     1, "outbytes":     0},
      81:    {"vt":  0x98, "lr": 0x710002BB90, "inbytes":     1, "outbytes":     0},
  },
  '0x710002BC90': { # , vtable size 10, possible vtables [0x71001D98A0 10, 0x71001D99A0 10]
      0:     {"vt":  0x20, "lr": 0x71000100D4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100010B8C']},
      1:     {"vt":  0x28, "lr": 0x71000102B0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100016B1C']},
      2:     {"vt":  0x30, "lr": 0x710001048C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001BE24']},
      3:     {"vt":  0x38, "lr": 0x7100010668, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001CBEC']},
      4:     {"vt":  0x40, "lr": 0x7100010844, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001D4A4']},
      10:    {"vt":  0x50, "lr": 0x710000BFD8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000C3C8']},
      11:    {"vt":  0x58, "lr": 0x710000C1B4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000F3B8']},
      20:    {"vt":  0x60, "lr": 0x710002BE80, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710002C1C8']},
      21:    {"vt":  0x68, "lr": 0x710002C05C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710002A164']},
      1000:  {"vt":  0x48, "lr": 0x7100010A20, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100020318']},
  },
  '0x710002C1C8': { # b5beb6e18da7d991 '0(0;o0)1(0)2(0;o0)3(0)5(0)6(0)10(0)11(0)12(0)13(0)14(0)15(0;b22)16(0)17(0;b6)18(0)19(0)20(0;o0)25(0)30(0)31(0)40(0)50(2)51(2;b21)60(0)70(0)80(0)90(2)100(2;o1)101(0)102(0)110(0;b6)120(0)130(0)140(2)150(0)'
      0:     {"vt":  0x20, "func": 0x71000AB468, "lr": 0x710002C628, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      1:     {"vt":  0x28, "func": 0x71000AB4E8, "lr": 0x710002C804, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      2:     {"vt":  0x30, "func": 0x71000AB5B0, "lr": 0x710002C9E4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      3:     {"vt":  0x38, "func": 0x71000AB630, "lr": 0x710002CBC0, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      5:     {"vt":  0x40, "func": 0x71000AB6F8, "lr": 0x710002CDA4, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      6:     {"vt":  0x48, "func": 0x71000AB764, "lr": 0x710002CF50, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      10:    {"vt":  0x50, "func": 0x71000AB7D0, "lr": 0x710002D0F0, "inbytes":     0, "outbytes":     0},
      11:    {"vt":  0x58, "func": 0x71000AB82C, "lr": 0x710002D244, "inbytes":     0, "outbytes":     8},
      12:    {"vt":  0x60, "func": 0x71000AB848, "lr": 0x710002D3BC, "inbytes":     0, "outbytes":  0x10},
      13:    {"vt":  0x68, "func": 0x71000AB890, "lr": 0x710002D534, "inbytes":     0, "outbytes":     1},
      14:    {"vt":  0x70, "func": 0x71000AB8CC, "lr": 0x710002D6AC, "inbytes":     0, "outbytes":  0x10},
      15:    {"vt":  0x78, "func": 0x71000AB928, "lr": 0x710002D838, "inbytes":     0, "outbytes":     0, "buffers": [22]},
      16:    {"vt":  0x80, "func": 0x71000AB9A4, "lr": 0x710002D9C0, "inbytes":     0, "outbytes":     1},
      17:    {"vt":  0x88, "func": 0x71000ABA18, "lr": 0x710002DB4C, "inbytes":     0, "outbytes":     4, "buffers": [6]},
      18:    {"vt":  0x90, "func": 0x71000ABA9C, "lr": 0x710002DD04, "inbytes":     0, "outbytes":  0x10},
      19:    {"vt":  0x98, "func": 0x71000ABB08, "lr": 0x710002DE7C, "inbytes":     0, "outbytes":     4},
      20:    {"vt":  0xA0, "func": 0x71000ABB50, "lr": 0x710002DFF8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      25:    {"vt":  0xA8, "func": 0x71000ABBD0, "lr": 0x710002E1D8, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      30:    {"vt":  0xB0, "func": 0x71000ABC3C, "lr": 0x710002E380, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      31:    {"vt":  0xB8, "func": 0x71000ABCF8, "lr": 0x710002E560, "inbytes":     0, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      40:    {"vt":  0xC0, "func": 0x71000ABDB4, "lr": 0x710002E73C, "inbytes":     0, "outbytes":     8},
      50:    {"vt":  0xC8, "func": 0x71000ABDD0, "lr": 0x710002E8B4, "inbytes":     8, "outbytes":     0},
      51:    {"vt":  0xD0, "func": 0x71000ABE1C, "lr": 0x710002EA34, "inbytes":     8, "outbytes":     0, "buffers": [21]},
      60:    {"vt":  0xD8, "func": 0x71000ABE94, "lr": 0x710002EBD4, "inbytes":     0, "outbytes":     8},
      70:    {"vt":  0xE0, "func": 0x71000ABF14, "lr": 0x710002ED4C, "inbytes":     0, "outbytes":     8},
      80:    {"vt":  0xE8, "func": 0x71000ABFA4, "lr": 0x710002EEC0, "inbytes":     0, "outbytes":     0},
      90:    {"vt":  0xF0, "func": 0x71000ABFC4, "lr": 0x710002F01C, "inbytes":     8, "outbytes":     0, "ininterfaces": ['0x7100008730']},
      100:   {"vt":  0xF8, "func": 0x71000AC010, "lr": 0x710002F224, "inbytes":     8, "outbytes":     0, "inhandles": [1], "outinterfaces": ['0x710002FE40']},
      101:   {"vt": 0x100, "func": 0x71000AC040, "lr": 0x710002F428, "inbytes":     0, "outbytes":     0},
      102:   {"vt": 0x108, "func": 0x71000AC0C8, "lr": 0x710002F578, "inbytes":     0, "outbytes":     0},
      110:   {"vt": 0x110, "func": 0x71000AC100, "lr": 0x710002F6E0, "inbytes":     0, "outbytes":     8, "buffers": [6]},
      120:   {"vt": 0x118, "func": 0x71000AC124, "lr": 0x710002F8A0, "inbytes":     0, "outbytes":     2},
      130:   {"vt": 0x120, "func": 0x71000AC194, "lr": 0x710002FA28, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      140:   {"vt": 0x128, "func": 0x71000AC200, "lr": 0x710002FBCC, "inbytes":     8, "outbytes":     0},
      150:   {"vt": 0x130, "func": 0x71000AC280, "lr": 0x710002FD34, "inbytes":     0, "outbytes":     1},
  },
  '0x710002FE40': { # , vtable size 4, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4]
      1:     {"vt":  0x20, "lr": 0x7100030030, "inbytes":  0x48, "outbytes":     0},
      2:     {"vt":  0x28, "lr": 0x71000301C8, "inbytes":     0, "outbytes":  0x40},
      10:    {"vt":  0x30, "lr": 0x7100030370, "inbytes":     0, "outbytes":     0, "outhandles": [1]},
      20:    {"vt":  0x38, "lr": 0x7100030528, "inbytes":     8, "outbytes":     0, "buffers": [69]},
  },
  '0x7100030668': { # , vtable size 10, possible vtables [0x71001D98A0 10, 0x71001D99A0 10]
      0:     {"vt":  0x20, "lr": 0x71000100D4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100010B8C']},
      1:     {"vt":  0x28, "lr": 0x71000102B0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100016B1C']},
      2:     {"vt":  0x30, "lr": 0x710001048C, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001BE24']},
      3:     {"vt":  0x38, "lr": 0x7100010668, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001CBEC']},
      4:     {"vt":  0x40, "lr": 0x7100010844, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001D4A4']},
      10:    {"vt":  0x50, "lr": 0x710000BFD8, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000C3C8']},
      11:    {"vt":  0x58, "lr": 0x710000C1B4, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710000F3B8']},
      20:    {"vt":  0x60, "lr": 0x7100030858, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100030BA0']},
      21:    {"vt":  0x68, "lr": 0x7100030A34, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710002A164']},
      1000:  {"vt":  0x48, "lr": 0x7100010A20, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100020318']},
  },
  '0x7100030BA0': { # , vtable size 15, possible vtables [0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x7100030E68, "inbytes":     0, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x7100030FB8, "inbytes":     0, "outbytes":     0},
      2:     {"vt":  0x30, "lr": 0x710003110C, "inbytes":     0, "outbytes":     8},
      3:     {"vt":  0x38, "lr": 0x7100031284, "inbytes":     8, "outbytes":     0},
      4:     {"vt":  0x40, "lr": 0x71000313EC, "inbytes":     1, "outbytes":     0},
      5:     {"vt":  0x48, "lr": 0x7100031558, "inbytes":     4, "outbytes":     0},
      6:     {"vt":  0x50, "lr": 0x71000316C0, "inbytes":     1, "outbytes":     0},
      10:    {"vt":  0x58, "lr": 0x7100031828, "inbytes":     0, "outbytes":     0},
      11:    {"vt":  0x60, "lr": 0x7100031978, "inbytes":     0, "outbytes":     0},
      20:    {"vt":  0x68, "lr": 0x7100031ACC, "inbytes":     1, "outbytes":     0},
      21:    {"vt":  0x70, "lr": 0x7100031C38, "inbytes":     1, "outbytes":     0},
      30:    {"vt":  0x78, "lr": 0x7100031DA4, "inbytes":     1, "outbytes":     0},
      31:    {"vt":  0x80, "lr": 0x7100031F10, "inbytes":     0, "outbytes":     1},
      90:    {"vt":  0x88, "lr": 0x7100032088, "inbytes":     1, "outbytes":     0},
      101:   {"vt":  0x90, "lr": 0x71000321F0, "inbytes":     0, "outbytes":     0},
  },
  '0x71000322D8': { # , vtable size 4, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4]
      1:     {"vt":  0x20, "lr": 0x71000324FC, "inbytes":     8, "outbytes":     0, "buffers": [5]},
      2:     {"vt":  0x28, "lr": 0x71000326AC, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710001B400']},
      3:     {"vt":  0x30, "lr": 0x710003288C, "inbytes":     8, "outbytes":     0, "outinterfaces": ['0x7100008730']},
      4:     {"vt":  0x38, "lr": 0x7100032A84, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x7100026AD4']},
  },
  '0x7100032BF0': { # , vtable size 3, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x7100032DD0, "inbytes":     0, "outbytes":     0, "outinterfaces": ['0x710003322C']},
      1:     {"vt":  0x28, "lr": 0x7100032FA8, "inbytes":     0, "outbytes":     4},
      6:     {"vt":  0x30, "lr": 0x7100033120, "inbytes":     0, "outbytes":     1},
  },
  '0x710003322C': { # , vtable size 3, possible vtables [0x71001DAF18 4, 0x71001DD520 4, 0x71001DAEA8 4, 0x71001D9AA0 4, 0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5, 0x71001DA1A0 6, 0x71001DBEE8 6, 0x71001DA2C0 6, 0x71001D8640 6, 0x71001DC790 8, 0x71001D51F0 8, 0x71001DBE78 8, 0x71001D98A0 10, 0x71001D99A0 10, 0x71001DD488 13, 0x71001DD3F0 13, 0x71001DD340 16, 0x71001DCDF0 16, 0x71001DC800 16]
      0:     {"vt":  0x20, "lr": 0x7100033408, "inbytes":     8, "outbytes":     0},
      1:     {"vt":  0x28, "lr": 0x710003356C, "inbytes":     4, "outbytes":     4},
      2:     {"vt":  0x30, "lr": 0x7100033700, "inbytes":     1, "outbytes":     0},
  },
  '0x7100072B14': { # , vtable size 5, possible vtables [0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5]
      0:     {"vt":  0x20, "lr": 0x7100072D0C, "inbytes":     0, "outbytes":     4},
      1:     {"vt":  0x28, "lr": 0x7100072E94, "inbytes":     4, "outbytes":     0, "outhandles": [2]},
      2:     {"vt":  0x30, "lr": 0x7100073060, "inbytes":     0, "outbytes":     0, "outhandles": [2]},
      3:     {"vt":  0x38, "lr": 0x7100073204, "inbytes":     0, "outbytes":     2},
      4:     {"vt":  0x40, "lr": 0x710007338C, "inbytes":     4, "outbytes":     0, "outhandles": [2]},
  },
  '0x7100079C00': { # , vtable size 5, possible vtables [0x71001DBD10 5, 0x71001D8978 5, 0x71001D9488 5]
      32:    {"vt":  0x20, "lr": 0x7100079E14, "inbytes":  0x10, "outbytes":     0, "pid": True},
      201:   {"vt":  0x28, "lr": 0x7100079FEC, "inbytes":  0x10, "outbytes":  0x20, "buffers": [69], "pid": True},
      203:   {"vt":  0x30, "lr": 0x710007A248, "inbytes":  0x50, "outbytes":  0x20, "buffers": [69], "pid": True},
      205:   {"vt":  0x38, "lr": 0x710007A4D4, "inbytes":  0x50, "outbytes":  0x20, "buffers": [21, 69], "pid": True},
      210:   {"vt":  0x40, "lr": 0x710007A754, "inbytes":  0x50, "outbytes":  0x20, "buffers": [21, 69]},
  },
},
        "#,
        );
    }
}
