use crate::shared::i18n::{self, Locale};
use crate::shell::app::{Order, SearchType};

use super::{Command, Item};

pub fn search_types(current: SearchType) -> Vec<Item> {
    SearchType::ALL
        .into_iter()
        .map(|kind| Item::choice(kind.label(), kind == current, Command::SearchType(kind)))
        .collect()
}

pub fn playlist_order(current: Order) -> Vec<Item> {
    Order::ALL
        .into_iter()
        .map(|order| {
            Item::choice(
                order.label(),
                order == current,
                Command::PlaylistOrder(order),
            )
        })
        .collect()
}

pub fn channel_order(current: Order) -> Vec<Item> {
    Order::ALL
        .into_iter()
        .map(|order| {
            Item::choice(
                order.label(),
                order == current,
                Command::ChannelOrder(order),
            )
        })
        .collect()
}

pub fn languages(current: Locale) -> Vec<Item> {
    i18n::AVAILABLE
        .into_iter()
        .map(|locale| Item::choice(locale.name(), locale == current, Command::Locale(locale)))
        .collect()
}
