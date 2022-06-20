pub trait Writer {
    fn write_bytes(&mut self, data: &[u8]);

    fn write(&mut self, data: &(impl WriteAsBytes + ?Sized)) {
        data.write_as_bytes(self)
    }
}

struct CountingWriter(usize);

impl Writer for CountingWriter {
    #[inline]
    fn write_bytes(&mut self, data: &[u8]) {
        self.0 += data.len();
    }
}

pub trait WriteAsBytes {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized));

    #[inline]
    fn size(&self) -> usize {
        let mut writer = CountingWriter(0);

        writer.write(self);

        writer.0
    }
}

macro_rules! as_bytes_impl_transmute {
    ($t:ty) => {
        impl crate::conv_traits::WriteAsBytes for $t {
            fn write_as_bytes(&self, dest: &mut (impl crate::conv_traits::Writer + ?Sized)) {
                const SIZE: usize = ::core::mem::size_of::<$t>();

                let buffer: [u8; SIZE] = unsafe { ::core::mem::transmute_copy(self) };

                crate::conv_traits::Writer::write_bytes(dest, &buffer)
            }
        }
    };
}

pub(crate) use as_bytes_impl_transmute;
