use chrono::NaiveDate;
use crate::models::common::app_name::AppName;

#[derive(Debug, Clone)]
pub struct DateSelectionState {
    pub app_name: AppName,
    pub start_date: Option<NaiveDate>,
    pub step: SelectionStep,
}

#[derive(Debug, Clone)]
pub enum SelectionStep {
    SelectingStartMonth,
    SelectingStartDay { year: i32, month: u32 },
    SelectingEndMonth,
    SelectingEndDay { year: i32, month: u32 },
}
