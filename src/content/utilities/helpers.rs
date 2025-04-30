use chrono::Utc;
use chrono_tz::Asia::Jakarta;

pub fn now_jakarta_time() -> String {
    let now_utc = Utc::now();
    let jakarta_time = now_utc.with_timezone(&Jakarta);
    jakarta_time.format("%Y-%m-%d %H:%M:%S").to_string()
}