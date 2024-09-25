use std::{fs::File, io::{BufReader, Seek, Read}};
use anyhow::{Error, Result};
use ebml::Element;

mod ebml;
mod matroska;

fn is_doctype_matroska<T:Seek+Read>(elem:&Element, stream:&mut ebml::Stream<T>)->Result<bool>{
    if elem.id()!=ebml::EBMLELEMENT_ID {
        return Ok(false);
    }
    let l=stream.children(&elem)?;
    for e in l{
        match e.id() {
            ebml::DOCTYPE_ID=> {
                let doctype=stream.read_string(&e)?;
                if doctype=="matroska" {
                    return Ok(true);
                }
            }
            _=>{}
        }
    }
    Ok(false)
}

fn main()->Result<()> {
    let f=File::open("/home/yoda/git/matroska-test-files/test_files/test1.mkv")?;
    let reader=BufReader::new(f);
    let mut stream=ebml::Stream::new(reader);
    let elems=stream.root_elements()?;

    if elems.len()<2 {
        return Err(Error::msg("Not enough elements in file"));
    }
    
    if is_doctype_matroska(&elems[0], &mut stream)?==false {
        return Err(Error::msg("File does not contain a matroska stream"));
    }
    println!("Matroska file confirmed, proceeding to parsing");
    Ok(())
}
