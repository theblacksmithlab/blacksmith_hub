/// User state for The Viper Room Bot workflow
///
/// This enum tracks the current state of a user's interaction with the bot,
/// particularly when navigating through settings menus and performing actions
/// that require text input or multi-step processes.
#[derive(Debug, Clone, PartialEq)]
pub enum TheViperRoomBotUserState {
    /// User is not in any specific menu or workflow
    Idle,

    /// User is viewing the main settings menu
    InSettingsMenu,

    // Channel management states
    /// User is viewing the channels management menu with action buttons
    ChannelsMenuView,

    /// User is in the process of adding channels (expecting text input with channel usernames)
    ChannelsAdding,

    /// User is viewing their list of channels
    ChannelsViewing,

    /// User is in the process of deleting channels (expecting callback query with channel selection)
    ChannelsDeleting,

    // Podcast time configuration states
    /// User is viewing the podcast time configuration menu
    PodcastTimeMenuView,

    /// User is in the process of setting podcast delivery time (expecting text input with time)
    PodcastTimeSetting,
}

impl Default for TheViperRoomBotUserState {
    fn default() -> Self {
        Self::Idle
    }
}

impl TheViperRoomBotUserState {
    /// Check if user is in any settings-related state
    pub fn is_in_settings(&self) -> bool {
        !matches!(self, Self::Idle)
    }

    /// Check if user state expects text input
    pub fn expects_text_input(&self) -> bool {
        matches!(
            self,
            Self::ChannelsAdding | Self::PodcastTimeSetting
        )
    }

    /// Check if user is in channel management flow
    pub fn is_in_channel_management(&self) -> bool {
        matches!(
            self,
            Self::ChannelsMenuView | Self::ChannelsAdding | Self::ChannelsViewing | Self::ChannelsDeleting
        )
    }
}
