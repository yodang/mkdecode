#![allow(dead_code)]
use anyhow::{Context, Error, Result};
use std::io::{Read, Seek, SeekFrom};

pub const EBMLELEMENT_ID:u64=0x1A45DFA3;
pub const EBMLVERSION_ID:u64=0x4286;
pub const EBMLREADVERSION_ID:u64=0x42F7;
pub const EBMLMAXIDLENGTH_ID:u64=0x42F2;
pub const EBMLMAXSIZELENGTH_ID:u64=0x42F3;
pub const DOCTYPE_ID:u64=0x4282;
pub const DOCTYPEVERSION_ID:u64=0x4287;
pub const DOCTYPEREADVERSION_ID:u64=0x4285;
pub const DOCTYPEEXTENSION_ID:u64=0x4281;
pub const DOCTYPEEXTENSIONNAME_ID:u64=0x4283;
pub const DOCTYPEEXTENSIONVERSION_ID:u64=0x4284;
pub const CRC32_ID:u64=0xBF;
pub const VOID_ID:u64=0xEC;

#[derive(Debug)]
pub struct Element{
    id:u64,
    length:u64,
    data_offset:u64
}

impl Element {
    pub fn id(&self)->u64 {
        self.id
    }
    pub fn length(&self)->u64 {
        self.length
    }
}

pub struct Header {

}

pub struct Vint{
    length: u32,
    raw_val: u64
}
//Does not handle integers bigger than 8 bytes
impl Vint {
    pub fn raw(&self)->u64
    {
        self.raw_val
    }

    pub fn data(&self)-> u64
    {
        self.raw_val^(1<<(7*self.length))
    }

    pub fn size(&self)->u32
    {
        self.length
    }

    pub fn from_data(data: &[u8])-> Result<Self>
    {
        if data.len() == 0 {
            return Ok(Vint{
                length:0,
                raw_val:0
            });
        }
        let size=data[0].leading_zeros()+1;
        if size>8 {
            return Err(Error::msg("Integer is too big"));
        }

        let mut val=0;
        for i in 0..size {
            val |= (data[i as usize] as u64)<<((size-1-i)*8);
        }

        Ok(Vint{
            length:size,
            raw_val:val
        })
    }

    pub fn read(source: &mut dyn Read)->Result<Self> {
        let mut byte_buf:[u8;1]=[0];
        let read=source.read(&mut byte_buf).context("Failed to read initial Vint byte")?;
        if read==0 {
            return Ok(Vint{
                length:0,
                raw_val:0
            });
        }

        let size=byte_buf[0].leading_zeros()+1;
        if size>8 {
            return Err(Error::msg("Integer is too big"));
        }

        let mut val=(byte_buf[0] as u64)<<((size-1)*8);
        for i in 1..size {
            source.read(&mut byte_buf).context("Error reading Vint")?;
            val |= (byte_buf[0] as u64)<<((size-1-i)*8);
        }

        Ok(Vint{
            length:size,
            raw_val:val
        })
    }
}

#[test]
fn vint_new()->Result<()>
{
    let data=[0x1A, 0x45, 0xDF, 0xA3];

    let v=Vint::from_data(&data)?;
    assert_eq!(v.size(), 4);
    assert_eq!(v.data(), 0xA45DFA3);
    assert_eq!(v.raw(), 0x1A45DFA3);
    Ok(())
}

pub struct Stream<T:Seek+Read> {
    source:T
}

impl<T:Seek+Read> Stream<T> {
    pub fn children(&mut self, elem: &Element)->Result<Vec<Element>> {
        self.source.seek(SeekFrom::Start(elem.data_offset)).context("Cannot seek to offset")?;
        let mut list=vec![];
        while self.source.stream_position()?<elem.data_offset+elem.length {
            let e=self.next_element().context("Error reading an element")?;
            list.push(e);
        }

        Ok(list)
    }

    pub fn read_element_data(&mut self, elem: &Element)->Result<Vec<u8>> {
        let mut buffer=Vec::new();
        buffer.resize(elem.length() as usize, 0);
        self.source.seek(SeekFrom::Start(elem.data_offset)).context("Cannot seek to offset")?;
        self.source.read_exact(buffer.as_mut_slice())?;
        Ok(buffer)
    }

    pub fn read_unsigned_integer(&mut self, elem: &Element)->Result<u64> {
        let length=elem.length();
        if length>8 {
            return Err(Error::msg("Element too big for an integer"));
        }
        let mut data=vec![];
        data.resize(length as usize, 0);
        self.source.seek(SeekFrom::Start(elem.data_offset)).context("Cannot seek to offset")?;
        self.source.read_exact(data.as_mut_slice())?;

        let mut val=0;
        for i in 0..length {
            val|= (data[i as usize] as u64)<<(length-1-i);
        }

        Ok(val)
    }

    pub fn read_string(&mut self, elem: &Element)->Result<String> {
        if elem.length() == 0 {
            return Ok(String::new());
        }

        let mut v=vec![];
        v.resize(elem.length() as usize, 0);
        self.source.seek(SeekFrom::Start(elem.data_offset)).context("Cannot seek to offset")?;
        self.source.read_exact(v.as_mut_slice())?;
        let s=String::from_utf8(v)?;

        Ok(s)
    }

    //Warning the result of this function will be the next element after the last one read by the stream, it does not save the cursor position
    pub fn next_element(&mut self)->Result<Element>
    {
        let id=Vint::read(&mut self.source).context("Reading id")?;
        if id.length==0 {
            return Ok(Element{
                id:0,
                length:0,
                data_offset:0
            })
        }
        let length=Vint::read(&mut self.source).context("Reading length")?;
        let offset=self.source.stream_position().context("Getting data offset")?;
        self.source.seek(SeekFrom::Current(length.data().try_into().unwrap()))?;
        Ok(Element{
            id:id.raw(),
            length:length.data(),
            data_offset:offset
        })
    }

    pub fn new(source:T)->Self {
        Stream{
            source:source
        }
    }

    pub fn root_elements(&mut self)->Result<Vec<Element>>{
        self.source.seek(SeekFrom::Start(0))?;
        let mut v=vec![];
        loop {
            let e=self.next_element()?;
            if e.id()!=0 {
                v.push(e);
            }
            else {
                break;
            }
        }
        Ok(v)
    }

}