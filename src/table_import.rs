use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io;
use flate2::Compression;
use flate2::write::GzEncoder;
use rmp::encode::*;
use tempdir::TempDir;

pub struct TableImportWritableChunk {
    elms_in_row: Option<(u32, u32)>,
    file_path: String,
    tmp_dir: TempDir,
    write: GzEncoder<File>
}

#[allow(dead_code)]
pub struct TableImportReadableChunk {
    pub file_path: String,
    tmp_dir: TempDir
}

#[derive(Debug, Clone)]
pub struct UnmatchElementNumsError(Option<(u32, u32)>);

impl fmt::Display for UnmatchElementNumsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some((capacity, added)) => write!(f, "capacity:{}, added:{}", capacity, added),
            None => write!(f, "Not initialized yet")
        }
    }
}

impl Error for UnmatchElementNumsError {
    fn description(&self) -> &str {
        "The number of elements in the row is unexpeceted"
    }
}

#[derive(Debug)]
pub enum TableImportChunkError {
    IOError(io::Error),
    UnmatchElementNums(UnmatchElementNumsError),
    UnexpectedError(String),
    MsgpackValueWriteError(ValueWriteError)
}

impl From<UnmatchElementNumsError> for TableImportChunkError {
    fn from(err: UnmatchElementNumsError) -> Self {
        TableImportChunkError::UnmatchElementNums(err)
    }
}

impl From<ValueWriteError> for TableImportChunkError {
    fn from(err: ValueWriteError) -> Self {
        TableImportChunkError::MsgpackValueWriteError(err)
    }
}

impl From<io::Error> for TableImportChunkError {
    fn from(err: io::Error) -> Self {
        TableImportChunkError::IOError(err)
    }
}

impl fmt::Display for TableImportChunkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TableImportChunkError::IOError(ref x) => write!(f, "{}", x),
            TableImportChunkError::UnmatchElementNums(ref x) => write!(f, "{}", x),
            TableImportChunkError::UnexpectedError(ref x) => write!(f, "{}", x),
            TableImportChunkError::MsgpackValueWriteError(ref x) => write!(f, "{}", x)
        }
    }
}

impl Error for TableImportChunkError {
    fn description(&self) -> &str {
        match *self {
            TableImportChunkError::IOError(ref x) => x.description(),
            TableImportChunkError::UnmatchElementNums(ref x) => x.description(),
            TableImportChunkError::UnexpectedError(ref x) => x,
            TableImportChunkError::MsgpackValueWriteError(ref x) => x.description()
        }
    }
}

impl TableImportWritableChunk {
    pub fn new() -> Result<TableImportWritableChunk, TableImportChunkError> {
        // let uuid =  Uuid::new_v4().hyphenated().to_string();
        // let tmp_dir = try!(TempDir::new(format!("td-client-rust-{}", uuid).as_str()));
        let tmp_dir = try!(TempDir::new("td-client-rust"));
        let tmp_file_path = tmp_dir.path().join("msgpack.gz");
        let file_path = try!(tmp_file_path.
                             to_str().
                             ok_or(
                                 TableImportChunkError::UnexpectedError(
                                     format!("Failed to convert path to string: {:?}",
                                             tmp_file_path))
                                 )).to_string();
        let file = try!(File::create(file_path.clone()));
        let write = GzEncoder::new(file, Compression::Default);
        Ok(TableImportWritableChunk {
            elms_in_row: None,
            file_path: file_path,
            tmp_dir: tmp_dir,
            write: write
        })
    }

    fn check_elm_number(&self) -> Result<(), TableImportChunkError> {
        match self.elms_in_row {
            Some((capacity, added)) =>
                if capacity != added {
                    try!(Err(UnmatchElementNumsError(Some((capacity, added)))))
                },
            None => ()
        };
        Ok(())
    }

    pub fn next_row(&mut self, len: u32) -> Result<(), TableImportChunkError> {
        try!(self.check_elm_number());
        try!(write_map_len(&mut self.write, len));
        self.elms_in_row = Some((len, 0));
        Ok(())
    }

    fn incr_elms_in_row(&mut self) -> Result<(), UnmatchElementNumsError> {
        match self.elms_in_row {
            Some((capacity, added)) => {
                let new_added = added + 1;
                if capacity < new_added {
                    try!(Err(UnmatchElementNumsError(Some((capacity, new_added)))))
                }
                else {
                    self.elms_in_row = Some((capacity, new_added));
                    Ok(())
                }
            },
            None => try!(Err(UnmatchElementNumsError(None)))
        }
    }

    pub fn write_key_and_array_header(&mut self, key: &str, len: u32) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_array_len(&mut self.write, len));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_bin(&mut self, key: &str, data: &[u8]) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_bin(&mut self.write, data));
        try!(self.incr_elms_in_row());
        Ok(())
    }


    pub fn write_key_and_bool(&mut self, key: &str, val: bool) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_bool(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_ext_meta(&mut self, key: &str, len: u32, typeid: i8) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_ext_meta(&mut self.write, len, typeid));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_f32(&mut self, key: &str, val: f32) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_f32(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_f64(&mut self, key: &str, val: f64) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_f64(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_i16(&mut self, key: &str, val: i16) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_i16(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_i32(&mut self, key: &str, val: i32) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_i32(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_i64(&mut self, key: &str, val: i64) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_i64(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_i8(&mut self, key: &str, val: i8) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_i8(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_map_len(&mut self, key: &str, len: u32) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_map_len(&mut self.write, len));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_nfix(&mut self, key: &str, val: i8) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_nfix(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_nil(&mut self, key: &str) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_nil(&mut self.write));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_pfix(&mut self, key: &str, val: u8) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_pfix(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_sint(&mut self, key: &str, val: i64) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_sint(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_sint_eff(&mut self, key: &str, val: i64) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_sint(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_str(&mut self, key: &str, data: &str) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_str(&mut self.write, data));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_u16(&mut self, key: &str, val: u16) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_u16(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_u32(&mut self, key: &str, val: u32) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_u32(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_u64(&mut self, key: &str, val: u64) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_u64(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_u8(&mut self, key: &str, val: u8) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_u8(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn write_key_and_uint(&mut self, key: &str, val: u64) -> Result<(), TableImportChunkError> {
        try!(write_str(&mut self.write, key));
        try!(write_uint(&mut self.write, val));
        try!(self.incr_elms_in_row());
        Ok(())
    }

    pub fn close(self) -> Result<TableImportReadableChunk, TableImportChunkError> {
        try!(self.check_elm_number());
        try!(self.write.finish());
        Ok(TableImportReadableChunk {
            file_path: self.file_path,
            tmp_dir: self.tmp_dir
        })
    }
}
