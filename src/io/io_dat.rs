
use super::super::index;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::io::prelude::*;
use super::super::byteorder::{ReadBytesExt, LittleEndian};
use std::error::Error;
use ::flate2::write::DeflateDecoder;

pub enum ContentType {
//    Empty,
    Binary,
//    Model,
//    Texture,
}

impl ContentType {
    pub fn from(t: u32) -> ContentType {
        match t {
            1 => /*ContentType::Empty*/ unimplemented!(),
            2 => ContentType::Binary,
            3 => /*ContentType::Model */unimplemented!(),
            4 => /*ContentType::Texture */unimplemented!(),
            _ => panic!("Unknown type!")
        }
    }
}

pub struct DataInfo {
    pub header_length: u32,
    pub content_type: ContentType,
    pub uncompressed_size: u32,
    pub block_buffer_size: u32,
    pub num_blocks: u32
}

pub fn read_data_header(file: &mut File, index: &index::File) -> Result<DataInfo, ::FFXIVError> {
    let current_pos = file.seek(SeekFrom::Current(0)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    file.seek(SeekFrom::Start(index.data_offset as u64)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    let hlen = file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    let cont_type = ContentType::from(file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?);
    let un_size = file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    let block_buf_size = file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    let block_count = file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    file.seek(SeekFrom::Start(current_pos)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    Ok(
        DataInfo {
            header_length: hlen,
            content_type: cont_type,
            uncompressed_size: un_size,
            block_buffer_size: block_buf_size,
            num_blocks: block_count,
        }
    )
}

pub struct BlockTableEntry {
    offset: u32,
    block_size: u16,
    decompressed_size: u16
}

pub fn read_block_table(file: &mut File, index_file: &index::File, info: &DataInfo) -> Result<Vec<BlockTableEntry>, ::FFXIVError> {
    let current_pos = file.seek(SeekFrom::Current(0)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    file.seek(SeekFrom::Start(index_file.data_offset as u64)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    file.seek(SeekFrom::Current(24)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    let mut blocktable=
        Vec::<BlockTableEntry>::with_capacity(info.num_blocks as usize);

    for _ in 0..info.num_blocks {
        blocktable.push(
            BlockTableEntry {
                offset: file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?,
                block_size: file.read_u16::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?,
                decompressed_size: file.read_u16::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?
            }
        );
    };
    file.seek(SeekFrom::Start(current_pos)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    Ok(blocktable)
}

const BLOCK_MAGIC: u32 = 0x10;
const BLOCK_PADDING: u32 = 0x80;

pub fn read_compressed_block(file: &mut File, offset: u32, block_size: u16) -> Result<(Vec<u8>, bool), ::FFXIVError> {
    let current_pos = file.seek(SeekFrom::Current(0)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    file.seek(SeekFrom::Start(offset as u64)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    if file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))? != BLOCK_MAGIC {
        return Err(::FFXIVError::ReadingDat(Box::<Error>::from(::FFXIVError::MagicMissing)));
    }
    file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    let compressed_length = file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    let decompressed_length = file.read_u32::<LittleEndian>().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    let is_compressed = compressed_length < 32000;



    let final_length =
//        block_size as u32 - BLOCK_MAGIC;
        if is_compressed {
            if (block_size as u32 + BLOCK_MAGIC) % BLOCK_PADDING != 0 {
                compressed_length + BLOCK_PADDING - ((block_size as u32 - BLOCK_MAGIC) % BLOCK_PADDING)
            }
            else { compressed_length }
        }
        else {decompressed_length};


    let mut data = Vec::<u8>::with_capacity(final_length as usize);
    file.take(final_length as u64).read_to_end(&mut data).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    file.seek(SeekFrom::Start(current_pos)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    Ok((data, is_compressed))
}

pub fn decompress(compressed: &Vec<u8>, size: u32) -> Result<Vec<u8>, ::FFXIVError> {
    let mut decoded_data = Vec::<u8>::with_capacity(size as usize);
    let mut z = DeflateDecoder::new(decoded_data);
    z.write(&compressed[..]).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    decoded_data = z.finish().map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;
    Ok(decoded_data)
}

pub fn read_and_decompress(file: &mut File, info: &DataInfo,
                           index_file: &index::File,
                           block_table: &Vec<BlockTableEntry>) -> Result<Vec<u8>, ::FFXIVError> {


    let current_pos = file.seek(SeekFrom::Current(0)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    let mut file_data = Vec::<u8>::with_capacity(info.uncompressed_size as usize);

    let mut total_size: u32 = 0;
    for table_entry in block_table {
        let block_offset: u32 = index_file.data_offset
            + info.header_length
            + table_entry.offset;

        let mut compressed_block = read_compressed_block(file, block_offset, table_entry.block_size)?;
        if compressed_block.1 {
            let mut decomp_block =
                decompress(&compressed_block.0,
                           table_entry.decompressed_size as u32)?;
            total_size += decomp_block.len() as u32;
            file_data.append(&mut decomp_block);
        }
        else {
            total_size += compressed_block.0.len() as u32;
            file_data.append(&mut compressed_block.0);
        }

    };

    if total_size != info.uncompressed_size {
        return Err(::FFXIVError::ReadingDat(Box::new(::FFXIVError::Custom(format!("Total size was not equal to the uncompressed size!!! This is a fatal error!")))));
    }

    file.seek(SeekFrom::Start(current_pos)).map_err(|o| ::FFXIVError::ReadingDat(Box::<Error>::from(o)))?;

    Ok(file_data)
}