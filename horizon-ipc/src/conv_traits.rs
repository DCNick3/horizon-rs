pub trait Writer {
    fn write_bytes(&mut self, data: &[u8]);
    /// Align the buffer to [alignment] bytes, return number of alignment bytes inserted
    fn align(&mut self, alignment: usize) -> usize;

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

    #[inline]
    fn align(&mut self, alignment: usize) -> usize {
        let new_len = ((self.0 + alignment - 1) / alignment) * alignment;
        let need_align = new_len - self.0;

        self.0 = new_len;
        need_align
    }
}

impl Writer for Vec<u8> {
    #[inline]
    fn write_bytes(&mut self, data: &[u8]) {
        self.extend_from_slice(data)
    }

    #[inline]
    fn align(&mut self, alignment: usize) -> usize {
        let new_len = ((self.len() + alignment - 1) / alignment) * alignment;
        let need_align = new_len - self.len();

        for _ in 0..need_align {
            self.push(0)
        }

        need_align
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

use alloc::vec::Vec;
pub(crate) use as_bytes_impl_transmute;

as_bytes_impl_transmute!(u8);
as_bytes_impl_transmute!(u16);
as_bytes_impl_transmute!(u32);
as_bytes_impl_transmute!(u64);

as_bytes_impl_transmute!(i8);
as_bytes_impl_transmute!(i16);
as_bytes_impl_transmute!(i32);
as_bytes_impl_transmute!(i64);

as_bytes_impl_transmute!(());
