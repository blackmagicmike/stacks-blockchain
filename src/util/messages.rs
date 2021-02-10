use std::io::{Read, Write};
use std::mem;

use net::MAX_MESSAGE_LEN;

use crate::util::errors::NetworkError;

/// Helper trait for various primitive common that make up Stacks messages
pub trait StacksMessageCodec {
    /// serialize implementors _should never_ error unless there is an underlying
    ///   failure in writing to the `fd`
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), NetworkError>
    where
        Self: Sized;
    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<Self, NetworkError>
    where
        Self: Sized;
    /// Convenience for serialization to a vec.
    ///  this function unwraps any underlying serialization error
    fn serialize_to_vec(&self) -> Vec<u8>
    where
        Self: Sized,
    {
        let mut bytes = vec![];
        self.consensus_serialize(&mut bytes)
            .expect("BUG: serialization to buffer failed.");
        bytes
    }
}
macro_rules! impl_stacks_message_codec_for_int {
    ($typ:ty; $array:expr) => {
        impl StacksMessageCodec for $typ {
            fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), NetworkError> {
                fd.write_all(&self.to_be_bytes())
                    .map_err(NetworkError::WriteError)
            }
            fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<Self, NetworkError> {
                let mut buf = $array;
                fd.read_exact(&mut buf).map_err(NetworkError::ReadError)?;
                Ok(<$typ>::from_be_bytes(buf))
            }
        }
    };
}

impl_stacks_message_codec_for_int!(u8; [0; 1]);
impl_stacks_message_codec_for_int!(u16; [0; 2]);
impl_stacks_message_codec_for_int!(u32; [0; 4]);
impl_stacks_message_codec_for_int!(u64; [0; 8]);
impl_stacks_message_codec_for_int!(i64; [0; 8]);

impl<T> StacksMessageCodec for Vec<T>
where
    T: StacksMessageCodec + Sized,
{
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), NetworkError> {
        let len = self.len() as u32;
        write_next(fd, &len)?;
        for i in 0..self.len() {
            write_next(fd, &self[i])?;
        }
        Ok(())
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<Vec<T>, NetworkError> {
        read_next_at_most::<R, T>(fd, u32::max_value())
    }
}

pub fn write_next<T: StacksMessageCodec, W: Write>(
    fd: &mut W,
    item: &T,
) -> Result<(), NetworkError> {
    item.consensus_serialize(fd)
}

pub fn read_next<T: StacksMessageCodec, R: Read>(fd: &mut R) -> Result<T, NetworkError> {
    let item: T = T::consensus_deserialize(fd)?;
    Ok(item)
}

fn read_next_vec<T: StacksMessageCodec + Sized, R: Read>(
    fd: &mut R,
    num_items: u32,
    max_items: u32,
) -> Result<Vec<T>, NetworkError> {
    let len = u32::consensus_deserialize(fd)?;

    if max_items > 0 {
        if len > max_items {
            // too many items
            return Err(NetworkError::DeserializeError(format!(
                "Array has too many items ({} > {}",
                len, max_items
            )));
        }
    } else {
        if len != num_items {
            // inexact item count
            return Err(NetworkError::DeserializeError(format!(
                "Array has incorrect number of items ({} != {})",
                len, num_items
            )));
        }
    }

    if (mem::size_of::<T>() as u128) * (len as u128) > MAX_MESSAGE_LEN as u128 {
        return Err(NetworkError::DeserializeError(format!(
            "Message occupies too many bytes (tried to allocate {}*{}={})",
            mem::size_of::<T>() as u128,
            len,
            (mem::size_of::<T>() as u128) * (len as u128)
        )));
    }

    let mut ret = Vec::with_capacity(len as usize);
    for _i in 0..len {
        let next_item = T::consensus_deserialize(fd)?;
        ret.push(next_item);
    }

    Ok(ret)
}

pub fn read_next_at_most<R: Read, T: StacksMessageCodec + Sized>(
    fd: &mut R,
    max_items: u32,
) -> Result<Vec<T>, NetworkError> {
    read_next_vec::<T, R>(fd, 0, max_items)
}

pub fn read_next_exact<R: Read, T: StacksMessageCodec + Sized>(
    fd: &mut R,
    num_items: u32,
) -> Result<Vec<T>, NetworkError> {
    read_next_vec::<T, R>(fd, num_items, 0)
}
