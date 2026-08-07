//! Script-key → chapter-title mapping, sourced from the original game's
//! `list_windows_ja.tbl` chapter table. Used to caption save slots.
//! Scripts without a verified title fall back to the script key itself.

/// Human-readable chapter title for a script key, if known.
pub fn chapter_title(script: &str) -> Option<&'static str> {
    match script {
        "nar1_00" => Some("7F"),
        "nar1_01" => Some("银色的酷派"),
        "nar1_02" => Some("地图"),
        "nar1_04" => Some("翡翠色的海"),
        "nar1_05" => Some("一号公路"),
        "nar1_06" => Some("艾歌"),
        "nar1_07" => Some("水仙花"),
        "nar1_08" => Some("百石建筑公司"),
        "nar2_01" => Some("序章"),
        "nar2_02" => Some("百石建筑公司"),
        "nar2_03" => Some("15岁的夏天"),
        "nar2_04" => Some("香草冰淇淋"),
        "nar2_05" => Some("可乐饼"),
        "nar2_06" => Some("报应"),
        "nar2_07" => Some("粉红色的指甲油"),
        "nar2_08" => Some("天主教徒"),
        "nar2_09" => Some("凤梨树"),
        "nar2_10" => Some("八年的好酒"),
        "nar2_11" => Some("尼洛与阿洛伊斯"),
        "nar2_12" => Some("温柔"),
        "nar2_13" => Some("禁忌"),
        "nar2_14" => Some("一半的祈祷"),
        "nar2_15" => Some("Road star"),
        "nar2_16" => Some("天空"),
        "nar2_17" => Some("为了谁"),
        "nar2_18" => Some("魔法"),
        "nar2_19" => Some("尾声"),
        _ => None,
    }
}
