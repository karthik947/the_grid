use crate::tui::settings::SettingsForm;

pub const MIN_COLUMN_SPACING: u16 = 0;
pub const MAX_COLUMN_SPACING: u16 = 10;
pub const MIN_TABLE_COUNT: u16 = 1;
pub const MAX_TABLE_COUNT: u16 = 4;
pub const MIN_TABLE_SPACING: u16 = 0;
pub const MAX_TABLE_SPACING: u16 = 10;

pub fn adjust_column_spacing(settings: &mut SettingsForm, delta: i16) {
    settings.layout_column_spacing = clamp_u16(
        settings.layout_column_spacing,
        delta,
        MIN_COLUMN_SPACING,
        MAX_COLUMN_SPACING,
    );
}

pub fn adjust_table_count(settings: &mut SettingsForm, delta: i16) {
    settings.layout_table_count = clamp_u16(
        settings.layout_table_count,
        delta,
        MIN_TABLE_COUNT,
        MAX_TABLE_COUNT,
    );
}

pub fn adjust_table_spacing(settings: &mut SettingsForm, delta: i16) {
    settings.layout_table_spacing = clamp_u16(
        settings.layout_table_spacing,
        delta,
        MIN_TABLE_SPACING,
        MAX_TABLE_SPACING,
    );
}

fn clamp_u16(current: u16, delta: i16, min: u16, max: u16) -> u16 {
    let next = (current as i16).saturating_add(delta);
    if next <= min as i16 {
        return min;
    }
    if next >= max as i16 {
        return max;
    }
    next as u16
}
