// SPDX-License-Identifier: Apache-2.0
use crate::dom::*;

pub fn toolbar(workspace: &web_sys::Element) -> Result {
    let bar = child(
        workspace,
        "section",
        &[("class", "view-toolbar"), ("aria-label", "保存した一覧")],
        None,
    )?;
    let picker = child(&bar, "div", &[("class", "view-picker")], None)?;
    child(
        &picker,
        "label",
        &[("for", "record-view")],
        Some("表示する一覧"),
    )?;
    child(
        &picker,
        "select",
        &[("id", "record-view"), ("disabled", "")],
        None,
    )?;
    let actions = child(&bar, "div", &[("class", "view-actions")], None)?;
    child(
        &actions,
        "button",
        &[("id", "save-view"), ("disabled", "")],
        Some("この条件で一覧を保存"),
    )?;
    child(
        &actions,
        "button",
        &[("id", "edit-view"), ("disabled", "")],
        Some("一覧の設定"),
    )?;
    child(
        &actions,
        "button",
        &[("id", "reload-views"), ("disabled", "")],
        Some("一覧を再読込"),
    )?;
    child(
        &bar,
        "span",
        &[("id", "view-count"), ("class", "muted")],
        None,
    )?;
    child(
        workspace,
        "p",
        &[
            ("id", "view-status"),
            ("class", "status"),
            ("role", "status"),
        ],
        None,
    )?;
    child(
        workspace,
        "p",
        &[("id", "view-modified"), ("class", "muted")],
        None,
    )?;
    Ok(())
}

pub fn dialog(root: &web_sys::Element) -> Result {
    let dialog = child(
        root,
        "dialog",
        &[
            ("id", "view-dialog"),
            ("aria-labelledby", "view-dialog-title"),
        ],
        None,
    )?;
    let form = child(&dialog, "form", &[("id", "view-form")], None)?;
    let heading = child(&form, "div", &[("class", "dialog-heading")], None)?;
    child(
        &heading,
        "h2",
        &[("id", "view-dialog-title")],
        Some("一覧を保存"),
    )?;
    child(
        &heading,
        "button",
        &[
            ("type", "button"),
            ("class", "close-view"),
            ("aria-label", "一覧の設定を閉じる"),
        ],
        Some("×"),
    )?;
    child(&form, "label", &[("for", "view-name")], Some("一覧名"))?;
    child(
        &form,
        "input",
        &[
            ("id", "view-name"),
            ("required", ""),
            ("maxlength", "160"),
            ("placeholder", "例：未対応の問い合わせ"),
        ],
        None,
    )?;
    child(
        &form,
        "p",
        &[("class", "muted")],
        Some("下の条件と表示項目を保存します。検索欄の未適用の入力は含みません。"),
    )?;
    child(
        &form,
        "p",
        &[("id", "view-query"), ("class", "view-query")],
        None,
    )?;
    child(&form, "h3", &[], Some("表示する項目と順序"))?;
    child(
        &form,
        "p",
        &[("class", "muted")],
        Some(
            "1項目以上を選び、上下ボタンで順序を変えられます。レコード編集とCSVではすべての項目を扱います。",
        ),
    )?;
    child(&form, "div", &[("id", "view-columns")], None)?;
    child(
        &form,
        "p",
        &[("id", "view-error"), ("class", "error"), ("role", "alert")],
        None,
    )?;
    child(
        &form,
        "button",
        &[("id", "reload-view"), ("type", "button"), ("hidden", "")],
        Some("最新版を読み込む（入力内容を置き換えます）"),
    )?;
    let actions = child(&form, "div", &[("class", "dialog-actions")], None)?;
    child(
        &actions,
        "button",
        &[
            ("id", "delete-view"),
            ("type", "button"),
            ("class", "danger"),
            ("hidden", ""),
        ],
        Some("一覧を削除"),
    )?;
    child(&actions, "span", &[("class", "spacer")], None)?;
    child(
        &actions,
        "button",
        &[("type", "button"), ("class", "close-view")],
        Some("キャンセル"),
    )?;
    child(
        &actions,
        "button",
        &[
            ("id", "confirm-view"),
            ("type", "submit"),
            ("class", "primary"),
        ],
        Some("一覧を保存"),
    )?;
    Ok(())
}
