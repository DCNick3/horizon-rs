pub trait Writer {
    fn write_bytes(&mut self, data: &[u8]);
    /// Align the buffer to [alignment] bytes, return number of alignment bytes inserted
    fn align(&mut self, alignment: usize) -> usize;

    fn write(&mut self, data: &(impl WriteAsBytes + ?Sized)) {
        data.write_as_bytes(self)
    }
}

pub trait Reader<'d> {
    fn read_bytes(&mut self, size: usize) -> &'d [u8];

    /// Align the buffer to [alignment] bytes, return number of alignment bytes inserted
    fn align(&mut self, alignment: usize) -> usize;

    fn read<T: ReadFromBytes<'d>>(&mut self) -> T {
        T::read_from_bytes(self)
    }
}

pub struct CountingWriter(usize);

impl CountingWriter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn count(&self) -> usize {
        self.0
    }
}

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

pub struct SliceWriter<'d> {
    leftover: &'d mut [u8],
    pos: usize,
}

impl<'d> SliceWriter<'d> {
    pub fn new(buffer: &'d mut [u8]) -> Self {
        Self {
            leftover: buffer,
            pos: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }
}

impl<'d> Writer for SliceWriter<'d> {
    #[inline]
    fn write_bytes(&mut self, data: &[u8]) {
        // do a trick to make the compiler sure we would not have multuple refs to the buffer
        // if we did not replace the leftover with &mut [],
        //   after the split_at_mut there would be aliasing references to the buffer
        let left = core::mem::replace(&mut self.leftover, &mut []);

        let (write, left) = left.split_at_mut(data.len());

        write.copy_from_slice(data);

        self.leftover = left;
        self.pos += data.len();
    }

    #[inline]
    fn align(&mut self, alignment: usize) -> usize {
        let left = core::mem::replace(&mut self.leftover, &mut []);

        let new_pos = ((self.pos + alignment - 1) / alignment) * alignment;
        let need_align = new_pos - self.pos;

        let (align, left) = left.split_at_mut(need_align);

        align.fill(0);

        self.leftover = left;
        self.pos = new_pos;

        need_align
    }
}

pub struct SliceReader<'d> {
    leftover: &'d [u8],
    pos: usize,
}

impl<'d> SliceReader<'d> {
    pub fn new(slice: &'d [u8]) -> Self {
        Self {
            pos: 0,
            leftover: slice,
        }
    }
}

impl<'d> Reader<'d> for SliceReader<'d> {
    fn read_bytes(&mut self, size: usize) -> &'d [u8] {
        let (ret, left) = self.leftover.split_at(size);

        self.leftover = left;
        self.pos += size;

        ret
    }

    fn align(&mut self, alignment: usize) -> usize {
        let new_pos = ((self.pos + alignment - 1) / alignment) * alignment;
        let need_align = new_pos - self.pos;

        self.leftover = &self.leftover[need_align..];
        self.pos = new_pos;

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

pub trait ReadFromBytes<'d> {
    fn read_from_bytes(src: &mut (impl Reader<'d> + ?Sized)) -> Self;
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

macro_rules! from_bytes_impl_transmute {
    ($t:ty) => {
        impl<'d> crate::conv_traits::ReadFromBytes<'d> for $t {
            fn read_from_bytes(src: &mut (impl crate::conv_traits::Reader<'d> + ?Sized)) -> Self {
                const SIZE: usize = ::core::mem::size_of::<$t>();

                let buffer: &'d [u8] = crate::conv_traits::Reader::read_bytes(src, SIZE);
                let buffer = <&[u8; SIZE]>::try_from(buffer).unwrap();

                unsafe { ::core::mem::transmute_copy(buffer) }
            }
        }
    };
}

pub(crate) use as_bytes_impl_transmute;
pub(crate) use from_bytes_impl_transmute;

as_bytes_impl_transmute!(u8);
as_bytes_impl_transmute!(u16);
as_bytes_impl_transmute!(u32);
as_bytes_impl_transmute!(u64);

as_bytes_impl_transmute!(i8);
as_bytes_impl_transmute!(i16);
as_bytes_impl_transmute!(i32);
as_bytes_impl_transmute!(i64);

as_bytes_impl_transmute!(());

from_bytes_impl_transmute!(u8);
from_bytes_impl_transmute!(u16);
from_bytes_impl_transmute!(u32);
from_bytes_impl_transmute!(u64);

from_bytes_impl_transmute!(i8);
from_bytes_impl_transmute!(i16);
from_bytes_impl_transmute!(i32);
from_bytes_impl_transmute!(i64);

from_bytes_impl_transmute!(());
