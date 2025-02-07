pub struct ResourcesDialogState {
    awaiting_option_choice: bool,
    awaiting_edit_type: bool,
    awaiting_show_type: bool,
    edit_type: EditType,
    show_type: ShowType,
    awaiting_data_entry: bool,
    awaiting_ask_message: bool,
}

#[derive(PartialEq, Eq)]
pub enum EditType {
    None,
    UsersToWhiteList,
    UsersToBlackList,
    Words,
}

#[derive(PartialEq, Eq)]
pub enum ShowType {
    None,
    UsersToWhiteList,
    UsersToBlackList,
    Words,
}
