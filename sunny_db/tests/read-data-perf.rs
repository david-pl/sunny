use serde::{Deserialize, Serialize};
use sunny_db::timeseries_db;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct PowerValues {
    power_pv: f64,
    power_grid: f64,
    power_used: f64,
}

#[test]
fn read_test() {
    // test to profile data reads with flamegraph
    // generate some data beforehand and put them in the right directory!
    let test_db_path = "./tests/stress-test-data";

    let tiny_db =
        timeseries_db::SunnyDB::<PowerValues>::new(200, &test_db_path, 2, 20);

    for _ in 0..2 {
        tiny_db.get_all_values();
    }

}
