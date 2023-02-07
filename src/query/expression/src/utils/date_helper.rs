// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::Date;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Duration;
use chrono::LocalResult;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono::Offset;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use chrono_tz::Tz;
use common_exception::ErrorCode;
use common_exception::Result;
use num_traits::AsPrimitive;
use once_cell::sync::Lazy;

use crate::types::date::check_date;
use crate::types::timestamp::check_timestamp;
use crate::types::timestamp::MICROS_IN_A_SEC;

#[derive(Debug, Clone, Copy)]
pub struct TzLUT {
    pub tz: Tz,
    pub offset_round_hour: bool,
    pub offset_round_minute: bool,
}

impl Default for TzLUT {
    fn default() -> Self {
        Self {
            tz: Tz::UTC,
            offset_round_hour: true,
            offset_round_minute: true,
        }
    }
}

static TZ_FACTORY: Lazy<TzFactory> = Lazy::new(|| {
    let factory = TzFactory {
        luts: dashmap::DashMap::new(),
    };
    let _ = factory.get(Tz::UTC);
    let _ = factory.get(Tz::Asia__Shanghai);
    let _ = factory.get(Tz::Asia__Tokyo);
    let _ = factory.get(Tz::America__New_York);
    let _ = factory.get(Tz::Europe__London);
    factory
});

pub struct TzFactory {
    pub luts: dashmap::DashMap<String, TzLUT>,
}

impl TzFactory {
    pub fn instance() -> &'static TzFactory {
        &TZ_FACTORY
    }

    pub fn get_by_name(&self, tz_name: &str) -> Result<TzLUT> {
        if let Some(lut) = self.luts.get(tz_name) {
            return Ok(*lut.value());
        }

        let tz = tz_name.parse::<Tz>().map_err(|_| {
            ErrorCode::InvalidTimezone("Timezone has been checked and should be valid")
        })?;
        let lut = TzLUT::new(tz);
        self.luts.insert(tz_name.to_string(), lut);
        Ok(lut)
    }

    pub fn get(&self, tz: Tz) -> TzLUT {
        let tz_name = tz.name();
        if let Some(lut) = self.luts.get(tz_name) {
            return *lut.value();
        }
        let lut = TzLUT::new(tz);
        self.luts.insert(tz_name.to_string(), lut);
        lut
    }
}

impl TzLUT {
    // it's very heavy to initial a TzLUT
    fn new(tz: Tz) -> Self {
        static DATE_LUT_MIN_YEAR: i32 = 1925;
        static DATE_LUT_MAX_YEAR: i32 = 2283;

        let mut offset_round_hour = true;
        let mut offset_round_minute = true;

        let date = NaiveDate::from_ymd(DATE_LUT_MIN_YEAR, 1, 1);
        let mut days = date.num_days_from_ce();

        loop {
            let time = NaiveDateTime::new(
                NaiveDate::from_num_days_from_ce(days),
                NaiveTime::from_hms(0, 0, 0),
            );
            if time.year() > DATE_LUT_MAX_YEAR {
                break;
            }

            days += 1;

            match tz.offset_from_local_datetime(&time) {
                LocalResult::Single(offset) => {
                    let offset = offset.fix();
                    if offset_round_hour && offset.local_minus_utc() % 3600 != 0 {
                        offset_round_hour = false;
                    }
                    if offset_round_minute && offset.local_minus_utc() % 60 != 0 {
                        offset_round_minute = false;
                    }
                }
                _ => {
                    continue;
                }
            }
        }
        Self {
            tz,
            offset_round_hour,
            offset_round_minute,
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn start_of_second(&self, ts: i64, seconds: i64) -> i64 {
        if seconds == 1 {
            return ts;
        }
        if seconds % 60 == 0 {
            return self.start_of_minutes(ts, seconds);
        }
        self.round_down(ts, seconds)
    }

    #[inline]
    fn start_of_minutes(&self, ts: i64, seconds_div: i64) -> i64 {
        if self.offset_round_minute {
            return if ts > 0 {
                ts / seconds_div * seconds_div
            } else {
                (ts + 1 - seconds_div) / seconds_div * seconds_div
            };
        }
        let datetime = self.tz.timestamp(ts, 0);

        let fix = datetime.offset().fix().local_minus_utc() as i64;
        fix + (ts - fix) / seconds_div * seconds_div
    }

    #[inline]
    fn round_down(&self, ts: i64, seconds_div: i64) -> i64 {
        if self.offset_round_hour {
            return if ts > 0 {
                ts / seconds_div * seconds_div
            } else {
                (ts + 1 - seconds_div) / seconds_div * seconds_div
            };
        }
        let datetime = self.tz.timestamp(ts, 0);
        let fix = datetime.offset().fix().local_minus_utc() as i64;
        fix + (ts - fix) / seconds_div * seconds_div
    }

    #[inline]
    pub fn round_timestamp_micros(&self, micros: i64, round: Round) -> i64 {
        self.round_timestamp(micros / MICROS_IN_A_SEC, round) * MICROS_IN_A_SEC
    }

    #[inline]
    pub fn round_timestamp(&self, ts: i64, round: Round) -> i64 {
        match round {
            Round::Second => ts,
            Round::Minute => self.start_of_minutes(ts, 60),
            Round::FiveMinutes => self.start_of_minutes(ts, 5 * 60),
            Round::TenMinutes => self.start_of_minutes(ts, 10 * 60),
            Round::FifteenMinutes => self.start_of_minutes(ts, 15 * 60),
            Round::TimeSlot => self.start_of_minutes(ts, 30 * 60),
            Round::Hour => self.round_down(ts, 3600),
            Round::Day => self.round_down(ts, 3600 * 24),
        }
    }

    pub fn to_minute(&self, ts: i64) -> u8 {
        if ts >= 0 && self.offset_round_hour {
            ((ts / 60) % 60) as u8
        } else {
            let datetime = self.tz.timestamp(ts, 0);
            datetime.minute() as u8
        }
    }

    pub fn to_hour(&self, ts: i64) -> u8 {
        let datetime = self.tz.timestamp(ts, 0);
        datetime.hour() as u8
    }
}

pub trait DateConverter {
    fn to_date(&self, tz: Tz) -> Date<Tz>;
    fn to_timestamp(&self, tz: Tz) -> DateTime<Tz>;
}

impl<T> DateConverter for T
where T: AsPrimitive<i64>
{
    fn to_date(&self, tz: Tz) -> Date<Tz> {
        let mut dt = tz.ymd(1970, 1, 1);
        dt = dt.checked_add_signed(Duration::days(self.as_())).unwrap();
        dt
    }

    fn to_timestamp(&self, tz: Tz) -> DateTime<Tz> {
        // Can't use `tz.timestamp_nanos(self.as_() * 1000)` directly, is may cause multiply with overflow.
        let micros = self.as_();
        let (mut secs, mut nanos) = (micros / MICROS_IN_A_SEC, (micros % MICROS_IN_A_SEC) * 1_000);
        if nanos < 0 {
            secs -= 1;
            nanos += 1_000_000_000;
        }
        tz.timestamp_opt(secs, nanos as u32).unwrap()
    }
}

// Timestamp arithmetic factors.
pub const FACTOR_HOUR: i64 = 3600;
pub const FACTOR_MINUTE: i64 = 60;
pub const FACTOR_SECOND: i64 = 1;
const LAST_DAY_LUT: [u8; 13] = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn add_years_base(year: i32, month: u32, day: u32, delta: i64) -> Result<NaiveDate, String> {
    let new_year = year + delta as i32;
    let mut new_day = day;
    if std::intrinsics::unlikely(month == 2 && day == 29) {
        new_day = last_day_of_year_month(new_year, month);
    }
    NaiveDate::from_ymd_opt(new_year, month, new_day).ok_or(format!(
        "Overflow on date YMD {}-{}-{}.",
        new_year, month, new_day
    ))
}

fn add_months_base(year: i32, month: u32, day: u32, delta: i64) -> Result<NaiveDate, String> {
    let total_months = month as i64 + delta - 1;
    let mut new_year = year + (total_months / 12) as i32;
    let mut new_month0 = total_months % 12;
    if new_month0 < 0 {
        new_year -= 1;
        new_month0 += 12;
    }

    // Handle month last day overflow, "2020-2-29" + "1 year" should be "2021-2-28", or "1990-1-31" + "3 month" should be "1990-4-30".
    let new_day = std::cmp::min::<u32>(
        day,
        last_day_of_year_month(new_year, (new_month0 + 1) as u32),
    );

    NaiveDate::from_ymd_opt(new_year, (new_month0 + 1) as u32, new_day).ok_or(format!(
        "Overflow on date YMD {}-{}-{}.",
        new_year,
        new_month0 + 1,
        new_day
    ))
}

// Get the last day of the year month, could be 28(non leap Feb), 29(leap year Feb), 30 or 31
fn last_day_of_year_month(year: i32, month: u32) -> u32 {
    let is_leap_year = NaiveDate::from_ymd_opt(year, 2, 29).is_some();
    if std::intrinsics::unlikely(month == 2 && is_leap_year) {
        return 29;
    }
    LAST_DAY_LUT[month as usize] as u32
}

macro_rules! impl_interval_year_month {
    ($name: ident, $op: expr) => {
        #[derive(Clone)]
        pub struct $name;

        impl $name {
            pub fn eval_date(
                date: i32,
                tz: TzLUT,
                delta: impl AsPrimitive<i64>,
            ) -> Result<i32, String> {
                let date = date.to_date(tz.tz);
                let new_date = $op(date.year(), date.month(), date.day(), delta.as_())?;
                check_date(
                    new_date
                        .signed_duration_since(NaiveDate::from_ymd(1970, 1, 1))
                        .num_days(),
                )
            }

            pub fn eval_timestamp(
                ts: i64,
                tz: TzLUT,
                delta: impl AsPrimitive<i64>,
            ) -> Result<i64, String> {
                let ts = ts.to_timestamp(tz.tz);
                let new_ts = $op(ts.year(), ts.month(), ts.day(), delta.as_())?;
                check_timestamp(NaiveDateTime::new(new_ts, ts.time()).timestamp_micros())
            }
        }
    };
}

impl_interval_year_month!(AddYearsImpl, add_years_base);
impl_interval_year_month!(AddMonthsImpl, add_months_base);

pub struct AddDaysImpl;

impl AddDaysImpl {
    pub fn eval_date(date: i32, delta: impl AsPrimitive<i64>) -> Result<i32, String> {
        check_date((date as i64).wrapping_add(delta.as_()))
    }

    pub fn eval_timestamp(date: i64, delta: impl AsPrimitive<i64>) -> Result<i64, String> {
        check_timestamp(date.wrapping_add(delta.as_() * 24 * 3600 * MICROS_IN_A_SEC))
    }
}

pub struct AddTimesImpl;

impl AddTimesImpl {
    pub fn eval_date(date: i32, delta: impl AsPrimitive<i64>, factor: i64) -> Result<i32, String> {
        check_date(
            (date as i64 * 3600 * 24 * MICROS_IN_A_SEC)
                .wrapping_add(delta.as_() * factor * MICROS_IN_A_SEC),
        )
    }

    pub fn eval_timestamp(
        ts: i64,
        delta: impl AsPrimitive<i64>,
        factor: i64,
    ) -> Result<i64, String> {
        check_timestamp(ts.wrapping_add(delta.as_() * factor * MICROS_IN_A_SEC))
    }
}

#[inline]
pub fn today_date() -> i32 {
    let now = Utc::now();
    NaiveDate::from_ymd(now.year(), now.month(), now.day())
        .signed_duration_since(NaiveDate::from_ymd(1970, 1, 1))
        .num_days() as i32
}

pub trait ToNumber<N> {
    fn to_number(dt: &DateTime<Tz>) -> N;
}

pub struct ToNumberImpl;

impl ToNumberImpl {
    pub fn eval_timestamp<T: ToNumber<R>, R>(ts: i64, tz: TzLUT) -> R {
        let dt = ts.to_timestamp(tz.tz);
        T::to_number(&dt)
    }

    pub fn eval_date<T: ToNumber<R>, R>(date: i32, tz: TzLUT) -> R {
        let dt = date.to_date(tz.tz).and_hms(0, 0, 0);
        T::to_number(&dt)
    }
}

pub struct ToYYYYMM;
pub struct ToYYYYMMDD;
pub struct ToYYYYMMDDHHMMSS;
pub struct ToYear;
pub struct ToMonth;
pub struct ToDayOfYear;
pub struct ToDayOfMonth;
pub struct ToDayOfWeek;
pub struct ToHour;
pub struct ToMinute;
pub struct ToSecond;

impl ToNumber<u32> for ToYYYYMM {
    fn to_number(dt: &DateTime<Tz>) -> u32 {
        dt.year() as u32 * 100 + dt.month()
    }
}

impl ToNumber<u32> for ToYYYYMMDD {
    fn to_number(dt: &DateTime<Tz>) -> u32 {
        dt.year() as u32 * 10_000 + dt.month() * 100 + dt.day()
    }
}

impl ToNumber<u64> for ToYYYYMMDDHHMMSS {
    fn to_number(dt: &DateTime<Tz>) -> u64 {
        dt.year() as u64 * 10_000_000_000
            + dt.month() as u64 * 100_000_000
            + dt.day() as u64 * 1_000_000
            + dt.hour() as u64 * 10_000
            + dt.minute() as u64 * 100
            + dt.second() as u64
    }
}

impl ToNumber<u16> for ToYear {
    fn to_number(dt: &DateTime<Tz>) -> u16 {
        dt.year() as u16
    }
}

impl ToNumber<u8> for ToMonth {
    fn to_number(dt: &DateTime<Tz>) -> u8 {
        dt.month() as u8
    }
}

impl ToNumber<u16> for ToDayOfYear {
    fn to_number(dt: &DateTime<Tz>) -> u16 {
        dt.ordinal() as u16
    }
}

impl ToNumber<u8> for ToDayOfMonth {
    fn to_number(dt: &DateTime<Tz>) -> u8 {
        dt.day() as u8
    }
}

impl ToNumber<u8> for ToDayOfWeek {
    fn to_number(dt: &DateTime<Tz>) -> u8 {
        dt.weekday().number_from_monday() as u8
    }
}

#[derive(Clone, Copy)]
pub enum Round {
    Second,
    Minute,
    FiveMinutes,
    TenMinutes,
    FifteenMinutes,
    TimeSlot,
    Hour,
    Day,
}

pub struct DateRounder;

impl DateRounder {
    pub fn eval_timestamp<T: ToNumber<i32>>(ts: i64, tz: TzLUT) -> i32 {
        let dt = ts.to_timestamp(tz.tz);
        T::to_number(&dt)
    }

    pub fn eval_date<T: ToNumber<i32>>(date: i32, tz: TzLUT) -> i32 {
        let dt = date.to_date(tz.tz).and_hms(0, 0, 0);
        T::to_number(&dt)
    }
}

/// Convert `chrono::DateTime` to `i32` in `Scalar::Date(i32)` for `DateType`.
///
/// It's the days since 1970-01-01.
#[inline]
fn datetime_to_date_inner_number(date: &DateTime<Tz>) -> i32 {
    date.naive_local()
        .signed_duration_since(NaiveDate::from_ymd(1970, 1, 1).and_hms(0, 0, 0))
        .num_days() as i32
}

pub struct ToLastMonday;
pub struct ToLastSunday;
pub struct ToStartOfMonth;
pub struct ToStartOfQuarter;
pub struct ToStartOfYear;
pub struct ToStartOfISOYear;

impl ToNumber<i32> for ToLastMonday {
    fn to_number(dt: &DateTime<Tz>) -> i32 {
        datetime_to_date_inner_number(dt) - dt.weekday().num_days_from_monday() as i32
    }
}

impl ToNumber<i32> for ToLastSunday {
    fn to_number(dt: &DateTime<Tz>) -> i32 {
        datetime_to_date_inner_number(dt) - dt.weekday().num_days_from_sunday() as i32
    }
}

impl ToNumber<i32> for ToStartOfMonth {
    fn to_number(dt: &DateTime<Tz>) -> i32 {
        datetime_to_date_inner_number(&dt.with_day(1).unwrap())
    }
}

impl ToNumber<i32> for ToStartOfQuarter {
    fn to_number(dt: &DateTime<Tz>) -> i32 {
        let new_month = dt.month0() / 3 * 3 + 1;
        datetime_to_date_inner_number(&dt.with_month(new_month).unwrap().with_day(1).unwrap())
    }
}

impl ToNumber<i32> for ToStartOfYear {
    fn to_number(dt: &DateTime<Tz>) -> i32 {
        datetime_to_date_inner_number(&dt.with_month(1).unwrap().with_day(1).unwrap())
    }
}

impl ToNumber<i32> for ToStartOfISOYear {
    fn to_number(dt: &DateTime<Tz>) -> i32 {
        let iso_year = dt.iso_week().year();
        let iso_dt = dt
            .timezone()
            .isoywd(iso_year, 1, chrono::Weekday::Mon)
            .and_hms(0, 0, 0);
        datetime_to_date_inner_number(&iso_dt)
    }
}
