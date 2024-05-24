use bitcode::{DecodeOwned, Encode};
use std::{cmp::Ordering, ops::{Add, Div, Mul, Sub}};

use crate::timeseries::TimeSeries;

pub trait TrapezoidalIntegral<T> {
    fn integrate(&self) -> Option<T>;
}

impl<T> TrapezoidalIntegral<T> for TimeSeries<T>
where
    T: Copy
        + Clone
        + Encode
        + DecodeOwned
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<f64, Output = T>,
{
    fn integrate(&self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let n = self.len();
        let entries = self.get_current_values();

        let (t_n, f_n) = entries[n - 1];
        let (t_0, f_0) = entries[0];
        let mut s = (f_n + f_0) * ((t_n - t_0) as f64);

        for i in 0..(n - 1) {
            let (t_i, val_i) = entries[i];
            let (t_ip1, val_ip1) = entries[i + 1];
            s = s + (val_i * (t_ip1 as f64) - val_ip1 * (t_i as f64));
        }

        Some(s * 0.5)
    }
}

pub trait MinMaxOfSeries<T> {
    fn min_by<F>(&self, f: F) -> Option<T>
    where
        F: FnMut(&T, &T) -> Ordering;

    fn max_by<F>(&self, f: F) -> Option<T>
        where
            F: FnMut(&T, &T) -> Ordering;
    // fn max(&self) -> Option<T>;
    // fn max_by_key<F>(&self, f: F) -> Option<T>
    // where
    //     F: FnMut(&T) -> T;

    // fn min(&self) -> Option<T>;
    // fn min_by_key<F>(&self, f: F) -> Option<T>
    // where
    //     F: FnMut(&T) -> T;
}

impl<T> MinMaxOfSeries<T> for TimeSeries<T>
where
    T: Copy + Clone + Encode + DecodeOwned,
{
    fn min_by<F>(&self, f: F) -> Option<T>
        where
            F: FnMut(&T, &T) -> Ordering {
        let values = self.get_current_values_without_time();
        values.into_iter().min_by(f)
    }

    fn max_by<F>(&self, f: F) -> Option<T>
        where
            F: FnMut(&T, &T) -> Ordering {
        let values = self.get_current_values_without_time();
        values.into_iter().min_by(f)
    }
    // fn max(&self) -> Option<T> {
    //     let values = self.get_current_values_without_time();
    //     values.into_iter().max()
    // }

    // fn max_by_key<F>(&self, f: F) -> Option<T>
    // where
    //     F: FnMut(&T) -> T,
    // {
    //     let values = self.get_current_values_without_time();
    //     values.into_iter().max_by_key(f)
    // }

    // fn min(&self) -> Option<T> {
    //     let values = self.get_current_values_without_time();
    //     values.into_iter().min()
    // }

    // fn min_by_key<F>(&self, f: F) -> Option<T>
    // where
    //     F: FnMut(&T) -> T,
    // {
    //     let values = self.get_current_values_without_time();
    //     values.into_iter().min_by_key(f)
    // }
}

pub trait Average<T> {
    fn average(&self) -> Option<T>;
}

impl<T> Average<T> for TimeSeries<T>
where
    T: Copy
        + Clone
        + Encode
        + DecodeOwned
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<f64, Output = T>
        + Div<f64, Output = T>,
{
    /// average over a continuous time series of values
    /// **NOTE**: this just calls integrate() from the TrapezoidalIntegral trait
    /// and divides by the interval; if you compute the integral at another place,
    /// consider re-using it and dividing outside to skip duplicate integration
    fn average(&self) -> Option<T> {
        let a = self.get_start_time()?;
        let b = self.get_end_time()?;
        let avg = self.integrate()? / ((b - a) as f64);
        Some(avg)
    }
}

// short-hand composite trait
pub trait Statistics<T>: TrapezoidalIntegral<T> + MinMaxOfSeries<T> + Average<T> {}

impl<T> Statistics<T> for TimeSeries<T> where
    T: Copy
        + Clone
        + Encode
        + DecodeOwned
        + Ord
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<f64, Output = T>
        + Div<f64, Output = T>
{
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integral() -> () {
        let mut ts = TimeSeries::<f64>::new(10);
        let times: Vec<u64> = vec![0, 10, 20, 30, 40];

        for i in 0..times.len() {
            ts.insert_value_at_time(times[i], i as f64);
        }

        let integral = ts.integrate().unwrap();

        assert_eq!(integral, 80.0);

        let m = ts.max_by(|a, b| b.partial_cmp(a).unwrap()).unwrap();

        assert_eq!(m, (times.len() - 1) as f64);
    }

}