use bitcode::{DecodeOwned, Encode};
use std::{
    cmp::Ordering,
    ops::{Add, Div, Mul, Sub},
};

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

        let (t_0, f_0) = entries[0];
        let (t_1, f_1) = entries[1];
        let mut s = (f_1 + f_0) * ((t_1 - t_0) as f64);

        for i in 1..(n - 1) {
            let (t_i, f_i) = entries[i];
            let (t_ip1, f_ip1) = entries[i + 1];
            s = s + (f_ip1 + f_i) * ((t_ip1 - t_i) as f64);
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
}

impl<T> MinMaxOfSeries<T> for TimeSeries<T>
where
    T: Copy + Clone + Encode + DecodeOwned,
{
    fn min_by<F>(&self, f: F) -> Option<T>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let values = self.get_current_values_without_time();
        values.into_iter().min_by(f)
    }

    fn max_by<F>(&self, f: F) -> Option<T>
    where
        F: FnMut(&T, &T) -> Ordering,
    {
        let values = self.get_current_values_without_time();
        values.into_iter().max_by(f)
    }
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
    fn test_statistics() -> () {
        let times: Vec<u64> = vec![0, 10, 20, 30, 40];

        // simplest case: linear with slope 1
        let mut ts = TimeSeries::<f64>::new(10);
        for i in 0..times.len() {
            ts.insert_value_at_time(times[i], i as f64);
        }
        let integral = ts.integrate().unwrap();
        assert_eq!(
            integral,
            ((times[times.len() - 1] - times[0]) as f64)
                * ts.get_current_values().last().unwrap().1
                * 0.5
        );

        let m = ts.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        assert_eq!(m, (times.len() - 1) as f64);

        // linear with slope != 1 and offset != 0
        let times: Vec<u64> = (2..100).collect();
        let mut ts = TimeSeries::<f64>::new(times.len());
        let k = 0.23;
        for i in 0..times.len() {
            ts.insert_value_at_time(times[i], k * times[i] as f64);
        }

        let integral = ts.integrate().unwrap();
        let d = integral
            - ((ts.get_current_values().last().unwrap().1
                + ts.get_current_values().first().unwrap().1)
                * ((times[times.len() - 1] - times[0]) as f64))
                / 2.0;
        let d_abs = if d < 0.0 { -d } else { d };
        assert!(d_abs < 0.0001);

        // non-linear case with offset == 0
        let times: Vec<u64> = (0..100).collect();
        fn f_nl(x: f64) -> f64 {
            2.0 * x - x * x
        }
        fn f_nl_integrated(x: f64) -> f64 {
            x * x - x * x * x / 3.0
        }

        let mut ts = TimeSeries::<f64>::new(times.len());
        for i in 0..times.len() {
            ts.insert_value_at_time(times[i], f_nl(times[i] as f64));
        }

        let integral = ts.integrate().unwrap();
        let definite_integral =
            f_nl_integrated(times[times.len() - 1] as f64) - f_nl_integrated(times[0] as f64);

        // difference, but normalized to the total value since that's pretty large and the approximation may not be that good
        let d = (integral - definite_integral) / integral;
        let d_abs = if d < 0.0 { -d } else { d };
        assert!(d_abs < 0.0001);

        assert_eq!(ts.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(), 1.0);

        assert_eq!(
            ts.min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            ts.get_current_values().last().unwrap().1
        );

        // test the average
        let avg1 = ts.average().unwrap();
        let avg2 = integral / (times[times.len() - 1] - times[0]) as f64;
        let d = avg1 - avg2;
        let d_abs = if d < 0.0 { -d } else { d };
        assert!(d_abs < 0.0001);
    }
}
