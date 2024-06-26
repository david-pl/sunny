use bitcode::{Decode, DecodeOwned, Encode};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Copy, Clone, Encode, Decode, PartialEq, Debug)]
struct TimeSeriesEntry<T> {
    time: u64,
    value: T,
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct TimeSeries<T> {
    init_size: usize,
    data: Vec<TimeSeriesEntry<T>>,
    start_time: Option<u64>,
    end_time: Option<u64>,
}

pub trait UnixTimestamp {
    fn timestamp(&self) -> u64;
}

impl UnixTimestamp for SystemTime {
    fn timestamp(&self) -> u64 {
        self.duration_since(UNIX_EPOCH)
            .expect("Time precedes unix epoch!")
            .as_millis() as u64 // we're not going beyond 500 Mio years
    }
}

impl<T: Copy + Encode + DecodeOwned> TimeSeries<T> {
    pub fn new(init_size: usize) -> Self {
        let data = Vec::<TimeSeriesEntry<T>>::with_capacity(init_size);
        TimeSeries {
            init_size: init_size,
            data: data,
            start_time: None,
            end_time: None,
        }
    }

    pub fn empty() -> Self {
        TimeSeries::<T>::new(0)
    }

    // getter methods for read only fields
    pub fn len(&self) -> usize {
        // TODO: should we implement the Allocator trait for this?
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.start_time.is_none() || self.data.is_empty()
    }

    pub fn get_start_time(&self) -> Option<u64> {
        self.start_time
    }

    pub fn get_end_time(&self) -> Option<u64> {
        self.end_time
    }

    pub fn get_current_values(&self) -> Vec<(u64, T)> {
        self.data
            .iter()
            .map(|entry| (entry.time, entry.value))
            .collect()
    }

    pub fn get_current_values_without_time(&self) -> Vec<T> {
        self.data.iter().map(|entry| entry.value).collect()
    }

    // private methods
    fn update_start_and_end(&mut self, time: u64) {
        match self.start_time {
            None => self.start_time = Some(time),
            Some(start) => {
                if start > time {
                    self.start_time = Some(time)
                }
            }
        }

        match self.end_time {
            None => self.end_time = Some(time),
            Some(end) => {
                if end < time {
                    self.end_time = Some(time)
                }
            }
        }
    }

    pub fn get_values_in_range(&self, start_time: u64, end_time: u64) -> Option<TimeSeries<T>> {
        if self.data.is_empty() {
            return None;
        }

        if end_time < self.start_time.unwrap() {
            // before any data points
            return None;
        }

        let start_index = self.find_last_index_after_time(start_time).unwrap_or(0);
        let end_index = self
            .find_last_index_after_time(end_time)
            .unwrap_or(self.data.len());

        let data = self.data[start_index..end_index].to_vec();

        if data.is_empty() {
            return None;
        }

        let new_series_start_time = data.first().map(|d| d.time);
        let new_series_end_time = data.last().map(|d| d.time);
        let tts = TimeSeries {
            init_size: data.len(),
            data: data,
            start_time: new_series_start_time,
            end_time: new_series_end_time,
        };

        Some(tts)
    }

    // adding values to the series
    pub fn insert_value_at_current_time(&mut self, value: T) {
        let now = SystemTime::now().timestamp();
        let entry = TimeSeriesEntry {
            time: now,
            value: value,
        };
        self.insert_entry(entry);
    }

    pub fn insert_value_at_time(&mut self, time: u64, value: T) {
        let entry = TimeSeriesEntry {
            time: time,
            value: value,
        };
        self.insert_entry(entry);
    }

    fn insert_entry(&mut self, entry: TimeSeriesEntry<T>) {
        let index = self.find_last_index_after_time(entry.time);
        match index {
            Some(idx) => self.data.insert(idx, entry),
            None => self.data.push(entry),
        };
        self.update_start_and_end(entry.time);
    }

    fn find_last_index_after_time(&self, time: u64) -> Option<usize> {
        if time < self.start_time? || time > self.end_time? {
            return None;
        }

        self.data
            .iter()
            .rposition(|entries| entries.time <= time)
            .map(|idx| idx + 1)
    }

    pub fn to_compressed_json(&self, level: i32) -> std::io::Result<Vec<u8>> {
        let bytes: &[u8] = &bitcode::encode(self);
        let output = zstd::stream::encode_all(bytes, level);
        output
    }

    pub fn from_compressed_json(compressed_json_bytes: &[u8]) -> anyhow::Result<TimeSeries<T>> {
        let bytes: &[u8] = &zstd::stream::decode_all(compressed_json_bytes)?;
        let ts = bitcode::decode(bytes)?;
        Ok(ts)
    }

    /// Appends one time series to another mutating the original time series
    /// **NOTE**: The timeseries to which you append *must* precede the timeseries
    /// that you are trying to append. Otherwise, this will cause a panic!
    pub fn append(&mut self, t: &TimeSeries<T>) -> &Self {
        if t.is_empty() {
            return self;
        }

        if self.start_time > t.start_time {
            panic!("Tried to append to timeseries in wrong order!")
        }

        if self.end_time > t.end_time {
            panic!("Tried to append to timeseries in wrong order!")
        }

        // update end time
        self.end_time = Some(t.end_time.unwrap());

        self.init_size = self.init_size + t.init_size;
        let mut data_to_append = t.data.clone();
        self.data.append(&mut data_to_append);
        self
    }
}
