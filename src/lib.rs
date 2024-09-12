//! View the contents of the Windows thumbnail cache files
//! 
//! <https://en.wikipedia.org/wiki/Windows_thumbnail_cache>
//! This library provides an easy-to-use function to read the contents of the thumbnail cache files and view the cache entries of it
//! Supports Windows Vista and above
//! TODO :
//! - Data and header verification


use std::{fs::{File, OpenOptions}, io::{Cursor, Read, Write}};

use thiserror::Error;

/// The Windows version associated with the thumbnail cache file
/// 
/// Thumbnail cache files can have different structures depending on its Windows version. This enum can provide the Windows version used for the file.
/// 
/// Note : Windows 10 also includes Windows 11.
#[derive(Clone, Copy, Debug)]
pub enum WindowsVersion {
    WinVista,
    Win7,
    Win8,
    Win81,
    Win10
}

#[derive(Clone, Copy, Debug)]
pub enum CacheType {
    Res16,
    Res32,
    Res48,
    Res96,
    Res256,
    Res768,
    Res1024,
    Res1280,
    Res1600,
    Res1920,
    Res2560,
    SR,
    Wide,
    EXIF,
    WideAlternate,
    CustomStream
}

/// These errors can appear if you're trying to read a file that isn't a thumbnail cache database or if you're trying to read an invalid file
#[derive(Error, Debug)]
pub enum ThumbsError {
    #[error("Invalid file, check the path again")]
    InvalidFile,
    #[error("Expected CMMM, got {0}. Are you sure you opened the right file?")]
    UnexpectedString(String),
    #[error("Invalid string. Are you sure you opened the right file?")]
    InvalidCheckString,
    #[error("An error occurred while trying to write a cache entry into a file or while trying to fill up a buffer while parsing.")]
    IoError(std::io::Error)
}

// Converts a slice into a slice with fixed length because some functions like to bitch about it.
// Thanks @malbarbo from Stackoverflow.
fn clone_into_array<A, T>(slice: &[T]) -> A
    where A: Sized + Default + AsMut<[T]>,
          T: Clone
{
    let mut a = Default::default();
    <A as AsMut<[T]>>::as_mut(&mut a).clone_from_slice(slice);
    a
}

/// Thumbscache
/// 
/// Represents the thumbscache database that is being read.
/// 
/// ```
/// use thumbscache::open_thumbscache;
/// fn main() {
///     let mut a = open_thumbscache(String::from("C:\\Users\\z\\AppData\\Local\\Microsoft\\Windows\\Explorer\\thumbcache_16.db"));
/// }
/// ```
/// 
/// The windows version and cache type stays None unless database gets parsed using the .read() function.
#[derive(Clone)]
pub struct Thumbscache {
    stream: Cursor<Vec<u8>>,
    pub windows_version: Option<WindowsVersion>,
    pub cache_entires: Vec<CacheEntry>,
    pub cache_type: Option<CacheType>
}

impl std::fmt::Debug for Thumbscache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Thumbscache").field("Windows version", &self.windows_version).field("Number of cache entries", &self.cache_entires.len()).field("Cache type", &self.cache_type).finish()
    }
}

/// Opens the thumbscache database and reads it to a struct.
/// Additional parsing is neccessary using the .read() function.
/// 
/// Returns an error if you specify an invalid file path
pub fn open_thumbscache(file: String) -> Result<Thumbscache, ThumbsError> {
    let mut bytes: Vec<u8> = Vec::new();
    if let Ok(mut opened_file) = std::fs::OpenOptions::new().read(true).open(file) {
        opened_file.read_to_end(&mut bytes).map_err(|x| {ThumbsError::IoError(x)})?;
        return Ok(Thumbscache {
            stream: Cursor::new(bytes),
            windows_version: None,
            cache_entires: Vec::new(),
            cache_type: None
        });
    }else {
        return Err(ThumbsError::InvalidFile);
    }
    

}

/// Cache entry
/// 
/// This struct represents a file in the thumbscache database. 
/// It includes the file extension of the file (only applicable for Windows Vista), the size of the data, the identifier string for it and the data itself, in .bmp format (unless stated otherwise in the file_extension field)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CacheEntry {
    size: u32,
    pub file_extension: Option<String>,
    identifier_string_size: u32,
    padding_size: u32,
    pub data_size: u32,
    data_checksum: u64,
    header_checksum: u64,
    pub identifier_string: String,
    pub data: Vec<u8>
}

impl CacheEntry {
    /// Writes the contents of the cache entry into a file.
    /// The file path defaults to the current directory unless stated otherwise.
    pub fn write_to_file(&self, file_path: Option<String>) -> Result<(), ThumbsError> {
        let mut file: File;
        if let Some(file_path) = file_path {
            if let Ok(opened_file) = OpenOptions::new().create(true).write(true).open(file_path) {
                file = opened_file;
            }else {
                return Err(ThumbsError::IoError(std::io::ErrorKind::InvalidInput.into())); 
            }
        }else {
            if let Ok(opened_file) = OpenOptions::new().create(true).write(true).open(format!("./{}.bmp",self.identifier_string)) {
                file = opened_file;
            }else {
                return Err(ThumbsError::IoError(std::io::ErrorKind::InvalidInput.into()));   
            }
        }
        if let Ok(_) = file.write_all(&self.data) {
            Ok(())
        }else {
            Err(ThumbsError::IoError(std::io::ErrorKind::InvalidData.into()))
        }
    }
} 

impl Thumbscache {
    /// Determines the Windows version and the cache type
    /// Reads all the cache entries and stores them into a list
    pub fn read(&mut self) -> Result<u32, ThumbsError> {
        let mut read_bytes: [u8; 32] = [0; 32];
        self.stream.read_exact(&mut read_bytes).map_err(|x| {ThumbsError::IoError(x)})?;
        if let Ok(check_string) = std::str::from_utf8(&read_bytes[0..4]) {
            if check_string != "CMMM" {
                return Err(ThumbsError::UnexpectedString(check_string.to_string()));
            }
        }else {
            return Err(ThumbsError::InvalidCheckString);
        }
        let format_version: u32 = u32::from_ne_bytes(clone_into_array(&read_bytes[4..8]));
        
        let cache_type: u32 = u32::from_ne_bytes(clone_into_array(&read_bytes[8..12]));
        match format_version {
            20 => {
                self.windows_version = Some(WindowsVersion::WinVista);
                match cache_type {
                    0 => self.cache_type = Some(CacheType::Res32),
                    1 => self.cache_type = Some(CacheType::Res96),
                    2 => self.cache_type = Some(CacheType::Res256),
                    3 => self.cache_type = Some(CacheType::Res1024),
                    4 => self.cache_type = Some(CacheType::SR),
                    _ => {}
                }
            },
            21 => {
                self.windows_version = Some(WindowsVersion::Win7);
                match cache_type {
                    0 => self.cache_type = Some(CacheType::Res32),
                    1 => self.cache_type = Some(CacheType::Res96),
                    2 => self.cache_type = Some(CacheType::Res256),
                    3 => self.cache_type = Some(CacheType::Res1024),
                    4 => self.cache_type = Some(CacheType::SR),
                    _ => {}
                }
            },
            30 => {
                self.windows_version = Some(WindowsVersion::Win8);
                match cache_type {
                    0 => self.cache_type = Some(CacheType::Res16),
                    1 => self.cache_type = Some(CacheType::Res32),
                    2 => self.cache_type = Some(CacheType::Res48),
                    3 => self.cache_type = Some(CacheType::Res96),
                    4 => self.cache_type = Some(CacheType::Res256),
                    5 => self.cache_type = Some(CacheType::Res1024),
                    6 => self.cache_type = Some(CacheType::SR),
                    7 => self.cache_type = Some(CacheType::Wide),
                    8 => self.cache_type = Some(CacheType::EXIF),
                    _ => {}
                }
            },
            31 => {
                self.windows_version = Some(WindowsVersion::Win81);
                match cache_type {
                    0 => self.cache_type = Some(CacheType::Res16),
                    1 => self.cache_type = Some(CacheType::Res32),
                    2 => self.cache_type = Some(CacheType::Res48),
                    3 => self.cache_type = Some(CacheType::Res96),
                    4 => self.cache_type = Some(CacheType::Res256),
                    5 => self.cache_type = Some(CacheType::Res1024),
                    6 => self.cache_type = Some(CacheType::Res1600),
                    7 => self.cache_type = Some(CacheType::SR),
                    8 => self.cache_type = Some(CacheType::Wide),
                    9 => self.cache_type = Some(CacheType::EXIF),
                    10 => self.cache_type = Some(CacheType::WideAlternate),
                    _ => {}
                }
            },
            32 => {
                self.windows_version = Some(WindowsVersion::Win10);
                match cache_type {
                    0 => self.cache_type = Some(CacheType::Res16),
                    1 => self.cache_type = Some(CacheType::Res32),
                    2 => self.cache_type = Some(CacheType::Res48),
                    3 => self.cache_type = Some(CacheType::Res96),
                    4 => self.cache_type = Some(CacheType::Res256),
                    5 => self.cache_type = Some(CacheType::Res768),
                    6 => self.cache_type = Some(CacheType::Res1280),
                    7 => self.cache_type = Some(CacheType::Res1920),
                    8 => self.cache_type = Some(CacheType::Res2560),
                    9 => self.cache_type = Some(CacheType::SR),
                    10 => self.cache_type = Some(CacheType::Wide),
                    11 => self.cache_type = Some(CacheType::EXIF),
                    12 => self.cache_type = Some(CacheType::WideAlternate),
                    13 => self.cache_type = Some(CacheType::CustomStream),
                    _ => {}
                }
            },
            _ => {}
        }
        let first_entry: u32 = u32::from_ne_bytes(clone_into_array(&read_bytes[12..16]));
        let _first_available_entry: u32 = u32::from_ne_bytes(clone_into_array(&read_bytes[16..20]));
        self.stream.set_position((24 + first_entry).into());
        let mut temp_bytes: [u8; 56];
        let mut padding_size: u32;
        let mut added_entries = 0;
        while self.stream.position() < self.stream.get_ref().len() as u64 {
            temp_bytes = [0;56];
            let _ = self.stream.read_exact(&mut temp_bytes);
            if let Ok(check_string) = std::str::from_utf8(&temp_bytes[0..4]) {
                if check_string != "CMMM" {
                    break;
                }
                match self.windows_version {
                    Some(version) => {
                        match version {
                            WindowsVersion::WinVista => {
                                let size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[4..8]));
                                let file_extension_vec_u16: Vec<u16> = temp_bytes[16..24].chunks_exact(2).into_iter().map(|a| u16::from_ne_bytes([a[0], a[1]])).collect();
                                let file_extension: String = String::from_utf16_lossy(&file_extension_vec_u16);
                                let identifier_string_size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[24..28]));
                                padding_size = u32::from_ne_bytes(clone_into_array(&temp_bytes[28..32]));
                                let data_size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[32..36]));
                                let data_checksum: u64 = u64::from_ne_bytes(clone_into_array(&temp_bytes[40..48]));
                                let header_checksum: u64 = u64::from_ne_bytes(clone_into_array(&temp_bytes[48..56]));
                                let mut identifier_string_vec: Vec<u8> = Vec::with_capacity(identifier_string_size as usize);
                                identifier_string_vec.extend_from_slice(&self.stream.get_ref()[self.stream.position() as usize..self.stream.position() as usize+identifier_string_size as usize]);
                                let identifier_string_vec_u16: Vec<u16> = identifier_string_vec.chunks_exact(2).into_iter().map(|a| u16::from_ne_bytes([a[0], a[1]])).collect();
                                let identifier_string: String = String::from_utf16_lossy(identifier_string_vec_u16.as_slice());
                                self.stream.set_position(self.stream.position() + padding_size as u64);
                                let mut data = vec![0u8; data_size.try_into().unwrap()];
                                self.stream.read_exact(&mut data).map_err(|x| {ThumbsError::IoError(x)})?;
                                // If we didn't read enough data then we skip to the next cache entry
                                self.stream.set_position(self.stream.position() + (size-(56+data_size+identifier_string_size+padding_size)) as u64);
                                let cache_entry = CacheEntry {
                                    size,
                                    file_extension: Some(file_extension),
                                    identifier_string_size,
                                    padding_size,
                                    data_size,
                                    data_checksum,
                                    header_checksum,
                                    identifier_string,
                                    data
                                };
                                self.cache_entires.push(cache_entry);      
                                added_entries = added_entries + 1;
                            },
                            WindowsVersion::Win7 => {
                                let size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[4..8]));
                                let identifier_string_size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[16..20]));
                                padding_size = u32::from_ne_bytes(clone_into_array(&temp_bytes[20..24]));
                                let data_size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[24..28]));
                                let data_checksum: u64 = u64::from_ne_bytes(clone_into_array(&temp_bytes[32..40]));
                                let header_checksum: u64 = u64::from_ne_bytes(clone_into_array(&temp_bytes[40..48]));
                                let mut identifier_string_vec: Vec<u8> = vec![0u8; identifier_string_size.try_into().unwrap()];
                                let _ = self.stream.read_exact(&mut identifier_string_vec);
                                let identifier_string_vec_u16: Vec<u16> = identifier_string_vec.chunks_exact(2).into_iter().map(|a| u16::from_ne_bytes([a[0], a[1]])).collect();
                                let identifier_string: String = String::from_utf16_lossy(identifier_string_vec_u16.as_slice());
                                self.stream.set_position(self.stream.position() + padding_size as u64);
                                let mut data = vec![0u8; data_size.try_into().unwrap()];
                                self.stream.read_exact(&mut data).map_err(|x| {ThumbsError::IoError(x)})?;
                                // If we didn't read enough data then we skip to the next cache entry
                                self.stream.set_position(self.stream.position() + (size-(56+data_size+identifier_string_size+padding_size)) as u64);
                                let cache_entry = CacheEntry {
                                    size,
                                    file_extension: None,
                                    identifier_string_size,
                                    padding_size,
                                    data_size,
                                    data_checksum,
                                    header_checksum,
                                    identifier_string,
                                    data
                                };
                                
                                self.cache_entires.push(cache_entry);      
                                added_entries = added_entries + 1;
                            },
                            _ => {
                                let size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[4..8]));
                                let identifier_string_size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[16..20]));
                                padding_size = u32::from_ne_bytes(clone_into_array(&temp_bytes[20..24]));
                                let data_size: u32 = u32::from_ne_bytes(clone_into_array(&temp_bytes[24..28]));
                                let data_checksum: u64 = u64::from_ne_bytes(clone_into_array(&temp_bytes[40..48]));
                                let header_checksum: u64 = u64::from_ne_bytes(clone_into_array(&temp_bytes[48..56]));
                                let mut identifier_string_vec: Vec<u8> = vec![0u8; identifier_string_size.try_into().unwrap()];
                                let _ = self.stream.read_exact(&mut identifier_string_vec);
                                let identifier_string_vec_u16: Vec<u16> = identifier_string_vec.chunks_exact(2).into_iter().map(|a| u16::from_ne_bytes([a[0], a[1]])).collect();
                                let identifier_string: String = String::from_utf16_lossy(identifier_string_vec_u16.as_slice());
                                self.stream.set_position(self.stream.position() + padding_size as u64);
                                let mut data = vec![0u8; data_size.try_into().unwrap()];
                                self.stream.read_exact(&mut data).map_err(|x| {ThumbsError::IoError(x)})?;
                                // If we didn't read enough data then we skip to the next cache entry
                                self.stream.set_position(self.stream.position() + (size-(56+data_size+identifier_string_size+padding_size)) as u64);
                                let cache_entry = CacheEntry {
                                    size,
                                    file_extension: None,
                                    identifier_string_size,
                                    padding_size,
                                    data_size,
                                    data_checksum,
                                    header_checksum,
                                    identifier_string,
                                    data
                                };
                                
                                self.cache_entires.push(cache_entry);      
                                added_entries = added_entries + 1;
                            }
                        }
                    },
                    None => {
                        
                    },
                }
                
            }
        }
        Ok(added_entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut a = open_thumbscache(String::from("C:\\Users\\z\\AppData\\Local\\Microsoft\\Windows\\Explorer\\thumbcache_16.db")).unwrap();
        a.read().unwrap();
        println!("{:?}",a);
    }
}