//! `docs/geo/household_hist.json` is what the twin's code path produces today: regenerated here
//! by the example's own `render` and compared byte for byte.
#[path = "../examples/household_hist.rs"]
mod household_hist;

#[test]
fn household_hist_json_is_current() {
    let have = include_str!("../../../docs/geo/household_hist.json");
    assert!(
        household_hist::render() == have,
        "docs/geo/household_hist.json is stale: run {}",
        household_hist::COMMAND
    );
}
