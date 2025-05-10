use chrono::TimeZone;

/// Returns the current sheet name or panics if no sheet is currently active.
pub fn get_current_sheet_name() -> &'static str {
    let current = chrono::offset::Local::now();

    let start3 = chrono::Local.with_ymd_and_hms(2025, 5, 6, 9, 0, 0).unwrap();
    let start4 = start3 + chrono::TimeDelta::weeks(1);
    let start5 = start4 + chrono::TimeDelta::weeks(1);
    let start6 = start5 + chrono::TimeDelta::weeks(1);
    let start7 = start6 + chrono::TimeDelta::weeks(1);
    let start8 = start7 + chrono::TimeDelta::weeks(1);
    let start9 = start8 + chrono::TimeDelta::weeks(1);
    let start10 = start9 + chrono::TimeDelta::weeks(1);
    let after = start10 + chrono::TimeDelta::weeks(1);

    if current > after {
        panic!("Terminate as no current sheet available.")
    }
    else if current > start10 {
        "sheet10"
    } else if current > start9 {
        "sheet09"
    } else if current > start8 {
        "sheet08"
    } else if current > start7 {
        "sheet07"
    } else if current > start6 {
        "sheet06"
    } else if current > start5 {
        "sheet05"
    } else if current > start4 {
        "sheet04"
    } else if current > start3 {
        "sheet03"
    } else {
        "sheet02"
    }
}
