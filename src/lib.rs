#[cfg(feature = "async")]
mod async_traits;

mod error;
mod int;
mod traits;

pub use crate::error::Error;
pub use crate::traits::{BinProtRead, BinProtSize, BinProtWrite};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::hash::Hash;
use std::io::{Read, Write};

/// This uses the "size-prefixed binary protocol".
/// https://ocaml.janestreet.com/ocaml-core/v0.13/doc/async_unix/Async_unix/Writer/index.html#val-write_bin_prot
pub fn binprot_write_with_size<W: Write, B: BinProtWrite>(b: &B, w: &mut W) -> std::io::Result<()> {
    let len = b.binprot_size();
    w.write_i64::<byteorder::LittleEndian>(len as i64)?;
    b.binprot_write(w)
}

/// This also uses the "size-prefixed binary protocol".
pub fn binprot_read_with_size<R: Read, B: BinProtRead>(r: &mut R) -> Result<B, Error> {
    // TODO: use the length value to avoid reading more that the specified number of bytes.
    let _len = r.read_i64::<byteorder::LittleEndian>()?;
    B::binprot_read(r)
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Nat0(pub u64);

impl BinProtWrite for Nat0 {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        int::write_nat0(w, self.0)
    }
}

impl BinProtWrite for i64 {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        int::write_i64(w, *self)
    }
}

impl BinProtWrite for f64 {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&self.to_le_bytes())
    }
}

impl BinProtWrite for () {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&[0u8])
    }
}

impl BinProtWrite for bool {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let b = if *self { 1 } else { 0 };
        w.write_all(&[b])
    }
}

impl<T: BinProtWrite> BinProtWrite for Option<T> {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        match &*self {
            None => w.write_all(&[0u8]),
            Some(v) => {
                w.write_all(&[1u8])?;
                v.binprot_write(w)
            }
        }
    }
}

impl<T: BinProtWrite> BinProtWrite for Vec<T> {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        int::write_nat0(w, self.len() as u64)?;
        for v in self.iter() {
            v.binprot_write(w)?
        }
        Ok(())
    }
}

impl<T: BinProtWrite> BinProtWrite for &[T] {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        int::write_nat0(w, self.len() as u64)?;
        for v in self.iter() {
            v.binprot_write(w)?
        }
        Ok(())
    }
}

impl BinProtWrite for String {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let bytes = self.as_bytes();
        int::write_nat0(w, bytes.len() as u64)?;
        w.write_all(&bytes)
    }
}

impl BinProtWrite for &str {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let bytes = self.as_bytes();
        int::write_nat0(w, bytes.len() as u64)?;
        w.write_all(&bytes)
    }
}

impl<K: BinProtWrite, V: BinProtWrite> BinProtWrite for std::collections::BTreeMap<K, V> {
    // The order is unspecified by the protocol
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        int::write_nat0(w, self.len() as u64)?;
        for (k, v) in self.iter() {
            k.binprot_write(w)?;
            v.binprot_write(w)?;
        }
        Ok(())
    }
}

impl<K: BinProtWrite, V: BinProtWrite> BinProtWrite for std::collections::HashMap<K, V> {
    // The order is unspecified by the protocol
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        int::write_nat0(w, self.len() as u64)?;
        for (k, v) in self.iter() {
            k.binprot_write(w)?;
            v.binprot_write(w)?;
        }
        Ok(())
    }
}

macro_rules! tuple_impls {
    ( $( $name:ident )+ ) => {
        impl<$($name: BinProtWrite),+> BinProtWrite for ($($name,)+)
        {
            #[allow(non_snake_case)]
            fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
                let ($($name,)+) = self;
                $($name.binprot_write(w)?;)+
                Ok(())
            }
        }

        impl<$($name: BinProtRead),+> BinProtRead for ($($name,)+)
        {
            #[allow(non_snake_case)]
            fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
            where
                Self: Sized,
            {
                $(let $name = $name::binprot_read(r)?;)+
                Ok(($($name,)+))
            }
        }
    };
}

tuple_impls! { A }
tuple_impls! { A B }
tuple_impls! { A B C }
tuple_impls! { A B C D }
tuple_impls! { A B C D E }
tuple_impls! { A B C D E F }
tuple_impls! { A B C D E F G }
tuple_impls! { A B C D E F G H }
tuple_impls! { A B C D E F G H I }

impl BinProtRead for Nat0 {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let u64 = int::read_nat0(r)?;
        Ok(Nat0(u64))
    }
}

impl BinProtRead for i64 {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let i64 = int::read_signed(r)?;
        Ok(i64)
    }
}

impl BinProtRead for f64 {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let f64 = r.read_f64::<LittleEndian>()?;
        Ok(f64)
    }
}

impl BinProtRead for () {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let c = r.read_u8()?;
        if c == 0 {
            Ok(())
        } else {
            Err(Error::UnexpectedValueForUnit(c))
        }
    }
}

impl BinProtRead for bool {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let c = r.read_u8()?;
        if c == 0 {
            Ok(false)
        } else if c == 1 {
            Ok(true)
        } else {
            Err(Error::UnexpectedValueForBool(c))
        }
    }
}

impl<T: BinProtRead> BinProtRead for Option<T> {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let c = r.read_u8()?;
        if c == 0 {
            Ok(None)
        } else if c == 1 {
            let v = T::binprot_read(r)?;
            Ok(Some(v))
        } else {
            Err(Error::UnexpectedValueForOption(c))
        }
    }
}

impl<T: BinProtRead> BinProtRead for Vec<T> {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let len = int::read_nat0(r)?;
        let mut v: Vec<T> = Vec::new();
        for _i in 0..len {
            let item = T::binprot_read(r)?;
            v.push(item)
        }
        Ok(v)
    }
}

impl<K: BinProtRead + Ord, V: BinProtRead> BinProtRead for std::collections::BTreeMap<K, V> {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let len = int::read_nat0(r)?;
        let mut res = std::collections::BTreeMap::new();
        for _i in 0..len {
            let k = K::binprot_read(r)?;
            let v = V::binprot_read(r)?;
            if res.insert(k, v).is_some() {
                return Err(Error::SameKeyAppearsTwiceInMap);
            }
        }
        Ok(res)
    }
}

impl<K: BinProtRead + Hash + Eq, V: BinProtRead> BinProtRead for std::collections::HashMap<K, V> {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let len = int::read_nat0(r)?;
        let mut res = std::collections::HashMap::new();
        for _i in 0..len {
            let k = K::binprot_read(r)?;
            let v = V::binprot_read(r)?;
            if res.insert(k, v).is_some() {
                return Err(Error::SameKeyAppearsTwiceInMap);
            }
        }
        Ok(res)
    }
}

impl BinProtRead for String {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let len = int::read_nat0(r)?;
        let mut buf: Vec<u8> = vec![0u8; len as usize];
        r.read_exact(&mut buf)?;
        let str = std::str::from_utf8(&buf)?;
        Ok(str.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithLen<T>(pub T);

impl<T: BinProtWrite + BinProtSize> BinProtWrite for WithLen<T> {
    fn binprot_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        let len = self.0.binprot_size();
        int::write_nat0(w, len as u64)?;
        self.0.binprot_write(w)
    }
}

impl<T: BinProtRead> BinProtRead for WithLen<T> {
    fn binprot_read<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error>
    where
        Self: Sized,
    {
        // TODO: stop reading past this length
        let _len = int::read_nat0(r)?;
        let t = T::binprot_read(r)?;
        Ok(WithLen(t))
    }
}
