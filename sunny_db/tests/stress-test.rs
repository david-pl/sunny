use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, UNIX_EPOCH};
use sunny_db::timeseries_db;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct PowerValues {
    power_pv: f64,
    power_grid: f64,
    power_used: f64,
}

#[test]
fn stress_test() {
    // generate a bunch of data
    let segment_size = 200;
    let segment_number = 2501;
    let test_db_path = "./tests/stress-test-data";

    let mut tiny_db =
        timeseries_db::SunnyDB::<PowerValues>::new(segment_size, &test_db_path, 2, 20);
    let mut rng = thread_rng();

    let now = Instant::now();
    for _i in 0..segment_number {
        for _j in 0..segment_size {
            let power_vals = PowerValues {
                power_pv: rng.gen_range(-1e3..1e3),
                power_grid: rng.gen_range(-1e3..1e3),
                power_used: rng.gen_range(-1e3..1e3),
            };
            tiny_db.insert_value_at_current_time(power_vals);
            // sleep here is required otherwise data loss occurs because we're writing too fast
            std::thread::sleep(Duration::from_nanos(1));
        }
    }

    // last segment should still be in memory
    assert_eq!(tiny_db.time_series.get_current_values().len(), segment_size);

    println!(
        "Elapsed time for writing {} segments of size {}: {} ms",
        segment_number,
        segment_size,
        now.elapsed().as_millis()
    );

    let now = Instant::now();
    let read_data = tiny_db.get_all_values();
    println!(
        "Elapsed time for reading {} segments of size {}: {} ms",
        segment_number,
        segment_size,
        now.elapsed().as_millis()
    );

    assert!(read_data.is_some(), "Found no data in time series DB!");

    assert_eq!(
        read_data.as_ref().unwrap().get_current_values().len(),
        segment_number * segment_size,
        "Fewer data points read than written!"
    );

    let time_series_start = read_data
        .unwrap()
        .get_start_time()
        .unwrap();
    let start_time = time_series_start + 80;
    let end_time = start_time + 50;

    let now = Instant::now();
    let few_values = tiny_db.get_values_in_range(start_time, end_time);
    let read_few_elapsed = now.elapsed().as_millis();

    assert!(few_values.is_some());
    assert!(few_values.as_ref().unwrap().len() >= 1);

    println!(
        "Elapsed time for reading {} values out of {}: {} ms",
        few_values.as_ref().unwrap().len(),
        segment_number * segment_size,
        read_few_elapsed
    );

    // clean up
    std::fs::remove_dir_all(&test_db_path).ok();
}

#[test]
fn test_data_loss() {
    let data_loss_path = "./tests/test-data-loss";
    let mut full_db_path = data_loss_path.to_owned();
    full_db_path.push_str("/data");
    let mut tiny_db = timeseries_db::SunnyDB::<PowerValues>::new(10, &data_loss_path, 2, 5);

    // write some values below loss threshold
    let mut rng = thread_rng();
    for _i in 0..4 {
        tiny_db.insert_value_at_current_time(PowerValues {
            power_pv: rng.gen_range(-1e3..1e3),
            power_grid: rng.gen_range(-1e3..1e3),
            power_used: rng.gen_range(-1e3..1e3),
        });
        std::thread::sleep(Duration::from_millis(10));
    }

    // nothing should be written, but kept in memory
    assert_eq!(tiny_db.time_series.len(), 4);
    tiny_db.lossy_persist();
    assert_eq!(tiny_db.time_series.len(), 4);

    let files: Vec<std::fs::DirEntry> = std::fs::read_dir(&full_db_path)
        .expect("Couldn't read data directory!")
        .map(|entry| entry.unwrap())
        .collect();

    assert!(files.is_empty());

    // add more values so we're above threshold, but below the cache size so it's not dumping automatically
    for _i in 0..4 {
        tiny_db.insert_value_at_current_time(PowerValues {
            power_pv: rng.gen_range(-1e3..1e3),
            power_grid: rng.gen_range(-1e3..1e3),
            power_used: rng.gen_range(-1e3..1e3),
        });
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(tiny_db.time_series.len(), 8);
    tiny_db.lossy_persist();
    assert_eq!(tiny_db.time_series.len(), 8);

    // should have a single file now
    let files: Vec<std::fs::DirEntry> = std::fs::read_dir(&full_db_path)
        .expect("Couldn't read data directory!")
        .map(|entry| entry.unwrap())
        .collect();
    assert_eq!(files.len(), 1);

    // write another set of values, should be dumped automatically now resulting in two files in total
    for _i in 0..4 {
        tiny_db.insert_value_at_current_time(PowerValues {
            power_pv: rng.gen_range(-1e3..1e3),
            power_grid: rng.gen_range(-1e3..1e3),
            power_used: rng.gen_range(-1e3..1e3),
        });
        std::thread::sleep(Duration::from_millis(10));
    }

    let files: Vec<std::fs::DirEntry> = std::fs::read_dir(&full_db_path)
        .expect("Couldn't read data directory!")
        .map(|entry| entry.unwrap())
        .collect();
    assert_eq!(files.len(), 2);

    assert_eq!(tiny_db.time_series.len(), 2);
    tiny_db.lossy_persist();
    assert_eq!(tiny_db.time_series.len(), 2);

    let files: Vec<std::fs::DirEntry> = std::fs::read_dir(&full_db_path)
        .expect("Couldn't read data directory!")
        .map(|entry| entry.unwrap())
        .collect();
    assert_eq!(files.len(), 2);

    std::fs::remove_dir_all(&data_loss_path).ok();
}
