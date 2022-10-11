//! # rust-strftime-wrapper
//!
//! This crate is a wrapper around strftime and strptime.
//! - It parses string date to Unix timestamp.
//! - It formats Unix timestamp into string date.
//!

// TODO:  docs & doc test

use std::{
    ffi::CString,
    fmt,
    os::raw::{c_char, c_int, c_long},
};

#[allow(non_camel_case_types)]
type c_time_t = i64;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *mut c_char,
}

impl Default for tm {
    fn default() -> Self {
        Self { 
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
            tm_zone: std::ptr::null_mut(),
         }
    }
}

extern "C" {
    fn gmtime_r(timestamp: *const c_time_t, tm: *mut tm) -> *mut tm;
    fn strftime(s: *mut c_char, maxsize: usize, format: *const c_char, timeptr: *const tm) -> usize;
    fn strptime(s: *const c_char, format: *const c_char, timeptr: *const tm) -> *mut c_char;
    fn mktime(timeptr: *mut tm) -> i64;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Error {
    TimestampToTmError,
    DateTimeParseError,
    TimestampOverflowError,
    FormatError
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TimestampToTmError => write!(f, "Error converting timestamp to C tm."),
            Error::DateTimeParseError => write!(f, "Error parsing date time."),
            Error::TimestampOverflowError => write!(f, "Timestamp overflow error"),
            Error::FormatError => write!(f, "Formatting error"),
        }
    }
}

impl std::error::Error for Error {}

/// Formats a timestamp in seconds to date time in the specified format.
pub fn strftime_format(timestamp: i64, format: impl AsRef<str>) -> Result<String, Error> {
    let format = format.as_ref();
    let mut tm = tm::default();
    if unsafe { gmtime_r(&timestamp, &mut tm as *mut tm) }.is_null() {
        return Err(Error::TimestampToTmError);
    }

    let format_len = format.len();
    let format = CString::new(format).map_err(|_| Error::FormatError)?;
    let mut buf_size = format_len;
    let mut buf: Vec<u8> = vec![0; buf_size];
    loop {
        let len = unsafe {
            strftime(
                buf.as_mut_ptr() as *mut c_char,
                buf_size,
                format.as_ptr() as *const c_char,
                &tm,
            )
        };
        if len == 0 {
            buf_size *= 2;
            buf.resize(buf_size, 0);
        } else {
            buf.truncate(len);
            return String::from_utf8(buf).map_err(|_| Error::FormatError);
        }
    }
}

/// Parses a string date time into timestamp in seconds using the specified format.
pub fn parse_strftime(date_time: impl AsRef<str>, format: impl AsRef<str>) -> Result<i64, Error> {
    let format = format.as_ref();
    let format = CString::new(format).map_err(|_| Error::FormatError)?;
    let date_time = date_time.as_ref();
    let date_time = CString::new(date_time).map_err(|_| Error::FormatError)?;

    let mut tm = tm::default();
    if unsafe {
        strptime(
            date_time.as_ptr() as *const c_char,
            format.as_ptr() as *const c_char,
            &mut tm as *mut tm,
        )
    }.is_null() {
        return Err(Error::DateTimeParseError);
    }
    // Use original value for error checking.
    // mktime does not make use of fields (tm_wday, tm_yday) to calculate time_t,
    // but if it succeeds, the value changes.
    tm.tm_yday = -1; 
    let timestamp = unsafe { mktime(&mut tm as *mut tm) };
    if timestamp == -1 && tm.tm_yday == -1 {
        return Err(Error::TimestampOverflowError);
    }
    
    return Ok(timestamp)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use crate::{parse_strftime, strftime_format};

    #[test]
    fn test_parse_strftime() {
        let timestamp = parse_strftime("1970-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let expected_timestamp = NaiveDateTime::parse_from_str("1970-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap().timestamp();
        assert_eq!(timestamp, expected_timestamp);

        let timestamp = parse_strftime("1969-12-31 23:59:59", "%Y-%m-%d %H:%M:%S").unwrap();
        let expected_timestamp = NaiveDateTime::parse_from_str("1969-12-31 23:59:59", "%Y-%m-%d %H:%M:%S").unwrap().timestamp();
        assert_eq!(timestamp, expected_timestamp); 

        let timestamp = parse_strftime("2022-11-22 10:12:30", "%Y-%m-%d %H:%M:%S").unwrap();
        let expected_timestamp = NaiveDateTime::parse_from_str("2022-11-22 10:12:30", "%Y-%m-%d %H:%M:%S").unwrap().timestamp();
        assert_eq!(timestamp, expected_timestamp); 
    }

    #[test]
    fn test_strftime_format() {
        let timestamp = NaiveDateTime::parse_from_str("1969-12-31 23:59:59", "%Y-%m-%d %H:%M:%S").unwrap().timestamp();
        let date_time = strftime_format(timestamp, "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!(date_time, "1969-12-31 23:59:59");
    }
}
