use crate::timeseries::{TimeSeries, UnixTimestamp};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, create_dir_all, remove_file, File};
use std::io::prelude::*;
use std::time::SystemTime;

pub struct SunnyDB<T> {
    pub time_series: TimeSeries<T>,
    time_series_cache_size: usize,
    data_path: String,
    /// The zstd compression level
    compression_level: i32,
    /// Specify at which point a time series segment should be written to disk when the database is closed
    data_loss_threshold: usize,
}

impl<T: Copy + DeserializeOwned + Serialize> SunnyDB<T> {
    pub fn new(
        time_series_cache_size: usize,
        dir_path: &str,
        compression_level: i32,
        data_loss_threshold: usize,
    ) -> Self {
        let data_dir_path = Self::init_directory(dir_path);

        let time_series = TimeSeries::<T>::new(time_series_cache_size);
        SunnyDB {
            time_series: time_series,
            time_series_cache_size: time_series_cache_size,
            data_path: data_dir_path,
            compression_level: compression_level,
            data_loss_threshold: data_loss_threshold,
        }
    }

    fn init_directory(dir_path: &str) -> String {
        let data_dir_path = if dir_path.ends_with('/') {
            dir_path.to_owned() + "data/"
        } else {
            dir_path.to_owned() + "/data/"
        };
        let dir = create_dir_all(&data_dir_path);

        match dir {
            Err(e) => panic!(
                "Error while trying to create database directory at {}. The error was: {}",
                dir_path, e
            ),
            _ => (),
        }

        let permission_file_path = data_dir_path.to_owned() + ".permission-check.tiny.db";

        let file = File::create(&permission_file_path);
        match file {
            Err(e) => panic!(
                "Error while trying to create a database file at {}. The error was: {}",
                data_dir_path, e
            ),
            Ok(_) => (),
        }

        // delete file again
        let delete_file = remove_file(permission_file_path);
        match delete_file {
            Err(e) => panic!(
                "Error while trying to delete database test file at {}. The error was: {}",
                data_dir_path, e
            ), //TODO: warn here? Could use tracing crate (already used by anyhow)
            _ => (),
        }

        return data_dir_path;
    }

    pub fn insert_value_at_current_time(&mut self, value: T) {
        self.time_series.insert_value_at_current_time(value);
        self.dump_time_series_if_full();
    }

    fn dump_time_series_if_full(&mut self) {
        if self.time_series.len() >= self.time_series_cache_size {
            match self.export_time_series_to_file() {
                Err(e) => panic!("Error while trying to dump time series: {}", e),
                Ok(()) => (), // TODO: log
            };
            self.time_series = TimeSeries::<T>::new(self.time_series_cache_size);
        }
    }

    /// persists the values currently in the time series without emptying the time series
    /// to prevent cluttering the DB with many small files, a threshold for the segment
    /// size is respected; this can be defined using the data_loss_threshold attribute
    pub fn lossy_persist(&mut self) {
        if self.data_loss_threshold < self.time_series.len() {
            self.export_time_series_to_file().ok();
        } else {
            println!("Warning: deliberately losing data on closing DB since there were only {} values in the time series and the threshold is set to {}", self.time_series.len(), self.data_loss_threshold);
        }
    }

    fn export_time_series_to_file(&self) -> Result<(), std::io::Error> {
        let start = self
            .time_series
            .get_start_time()
            .expect("Error: tried to export time series that has no start time set!");
        let end = self
            .time_series
            .get_end_time()
            .expect("Error: tried to export time series that has no end time set!");
        let file_name = format!("{}-{}", start, end);
        let mut file = File::create(self.data_path.to_owned() + &file_name)?;

        let data = self
            .time_series
            .to_compressed_json(self.compression_level)?;
        file.write_all(&data)
    }

    // getting values
    pub fn get_all_values(&self) -> Option<TimeSeries<T>> {
        // TODO: simplify by skipping search & everything
        let end_time = self
            .time_series
            .get_end_time()
            .unwrap_or(SystemTime::now().timestamp());
        self.get_values_in_range(0, end_time)
    }

    pub fn get_values_in_range(&self, start_time: u64, end_time: u64) -> Option<TimeSeries<T>> {
        if end_time < start_time {
            // someone accidentally switched start & end
            return self.get_values_in_range(end_time, start_time);
        }

        let ts_start_time = self.time_series.get_start_time();
        if ts_start_time.is_some() && ts_start_time.unwrap() <= start_time {
            // shortcut if all data is currently in memory anyway
            return self.time_series.get_values_in_range(start_time, end_time);
        }

        let read_data = self.read_persisted_data(start_time, end_time);

        if self.time_series.get_start_time() > Some(end_time) {
            // everything's been covered by reading the persisted data
            return read_data;
        }

        // part of it is in the time-series
        let ts = self
            .time_series
            .get_values_in_range(start_time, end_time)
            .unwrap_or(TimeSeries::<T>::empty());

        return match read_data {
            None => Some(ts),
            Some(mut d) => {
                d.append(&ts);
                Some(d)
            }
        };
    }

    fn read_persisted_data(&self, start_time: u64, end_time: u64) -> Option<TimeSeries<T>> {
        let mut files: Vec<fs::DirEntry> = fs::read_dir(&self.data_path)
            .expect("Couldn't read data directory!")
            .flatten()
            .collect();
        files.sort_by(|f1, f2| f1.path().cmp(&(f2.path())));
        let (start_index, end_index) =
            self.find_persisted_segment_index(&files, start_time, end_time);

        if start_index.is_none() && end_index.is_none() {
            // no data found
            return None;
        }

        // at least one entry was found in the files, so let's do what we can here
        let actual_start_index = start_index.unwrap_or(0);
        let actual_end_index = end_index.unwrap_or(files.len() - 1) + 1;

        let ts: Vec<TimeSeries<T>> = files[actual_start_index..actual_end_index]
            .into_iter()
            .map(|f| self.parse_file_to_timeseries(f))
            .flatten()
            .collect();

        // no data found apparently
        if ts.is_empty() {
            return None;
        }

        // only a single entry, which makes for a bit of a special case
        if ts.len() == 1 {
            return ts[0].get_values_in_range(start_time, end_time);
        }

        // multiple entries
        let mut t0 = ts[0]
            .get_values_in_range(start_time, end_time)
            .unwrap_or(TimeSeries::<T>::empty());

        if ts.len() > 2 {
            for t in &ts[1..(ts.len() - 1)] {
                t0.append(t);
            }
        }

        let t_n = ts[ts.len() - 1]
            .get_values_in_range(start_time, end_time)
            .unwrap_or(TimeSeries::<T>::empty());
        t0.append(&t_n);

        Some(t0)
    }

    fn find_persisted_segment_index(
        &self,
        files: &Vec<fs::DirEntry>,
        start_time: u64,
        end_time: u64,
    ) -> (Option<usize>, Option<usize>) {
        let segments: Vec<Option<(u64, u64)>> = files
            .iter()
            .map(|file| SunnyDB::<T>::parse_filename_to_times(file))
            .collect();

        // check if we're getting all the segments
        let first_segment = segments.iter().find(|&seg| seg.is_some());
        let last_segment = segments.iter().rev().find(|&seg| seg.is_some());
        if first_segment.is_none() && last_segment.is_none() {
            // no persisted data
            return (None, None);
        }

        if end_time < first_segment.unwrap().unwrap().0 {
            return (None, None);
        }

        let start_segment_index = if start_time < first_segment.unwrap().unwrap().0 {
            // starting from the very beginning
            Some(0)
        } else {
            segments.iter().position(|&seg| match seg {
                None => false,
                Some(s) => s.0 <= start_time,
            })
        };

        let end_segment_index = if end_time > last_segment.unwrap().unwrap().1 {
            Some(segments.len() - 1)
        } else {
            segments.iter().position(|&seg| match seg {
                None => false,
                Some(s) => s.0 <= end_time && end_time <= s.1,
            })
        };

        (start_segment_index, end_segment_index)
    }

    fn parse_filename_to_times(file: &fs::DirEntry) -> Option<(u64, u64)> {
        let file_name = file.file_name();
        let split_name: Vec<&str> = file_name.to_str()?.split("-").collect();
        if split_name.len() != 2 {
            return None;
        }

        let start_timestamp = match split_name[0].parse::<u64>() {
            Ok(t) => t,
            Err(_) => return None,
        };

        let end_timestamp = match split_name[1].parse::<u64>() {
            Ok(t) => t,
            Err(_) => return None,
        };

        Some((start_timestamp, end_timestamp))
    }

    fn parse_file_to_timeseries(&self, f: &fs::DirEntry) -> anyhow::Result<TimeSeries<T>> {
        let opened_file = File::open(f.path())?;
        let mut buf: Vec<u8> = vec![0; f.metadata()?.len() as usize];
        let _ = (&opened_file).read(&mut buf);
        TimeSeries::<T>::from_compressed_json(&buf)
    }
}
