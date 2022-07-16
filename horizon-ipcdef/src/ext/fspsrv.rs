use crate::fssrv::Path;
use core::str::Utf8Error;

const PATH_SIZE: usize = 0x300;

impl Path {
    pub fn new(s: impl AsRef<[u8]>) -> Self {
        let s = s.as_ref();
        assert!(s.len() <= PATH_SIZE);
        let mut r = Self { str: [0; 0x301] };

        r.str.copy_from_slice(s);

        r
    }

    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        core::str::from_utf8(self.as_ref())
    }
}

impl AsRef<[u8]> for Path {
    fn as_ref(&self) -> &[u8] {
        let (len, _) = self.str.iter().enumerate().find(|(_, p)| p == 0).unwrap();
        &self.str[..len]
    }
}
