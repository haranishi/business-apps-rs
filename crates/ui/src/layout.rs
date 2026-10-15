// SPDX-License-Identifier: Apache-2.0
//! The browser document is built with text APIs; no HTML template or injected HTML.
use crate::dom::{Result, append, child, document, element};
use wasm_bindgen::JsCast;

pub fn mount() -> Result {
    let body: web_sys::Element = document().body().expect("browser body").unchecked_into();
    // Retain the shell's loader script and noscript message while mounting the UI.
    let root = child(&body, "div", &[("id", "business-apps-root")], None)?;
    child(
        &root,
        "a",
        &[("class", "skip"), ("href", "#workspace")],
        Some("作業エリアへ"),
    )?;
    let header = child(&root, "header", &[("class", "topbar")], None)?;
    let brand = child(
        &header,
        "a",
        &[("class", "brand"), ("href", "/")],
        Some("business-apps"),
    )?;
    child(&brand, "span", &[], Some("-rs"))?;
    child(
        &header,
        "span",
        &[("class", "local-badge")],
        Some("この端末のワークスペース"),
    )?;
    let shell = child(&root, "div", &[("class", "shell")], None)?;
    let sidebar = child(
        &shell,
        "aside",
        &[("class", "sidebar"), ("aria-label", "アプリナビゲーション")],
        None,
    )?;
    let sidebar_heading = child(&sidebar, "div", &[("class", "sidebar-heading")], None)?;
    child(&sidebar_heading, "h2", &[], Some("アプリ"))?;
    child(
        &sidebar_heading,
        "button",
        &[("id", "create-app"), ("class", "primary"), ("disabled", "")],
        Some("＋ アプリを作る"),
    )?;
    child(
        &sidebar,
        "nav",
        &[("id", "app-nav"), ("aria-label", "アプリ一覧")],
        None,
    )?;
    child(
        &sidebar,
        "p",
        &[("class", "sidebar-note")],
        Some("項目を決めて、仕事のデータをひとつに。"),
    )?;
    let main = child(
        &shell,
        "main",
        &[("id", "workspace"), ("tabindex", "-1")],
        None,
    )?;
    child(
        &main,
        "p",
        &[
            ("id", "status"),
            ("class", "status"),
            ("role", "status"),
            ("aria-live", "polite"),
        ],
        Some("アプリを読み込み中…"),
    )?;
    welcome(&main)?;
    workspace(&main)?;
    app_dialog(&root)?;
    record_dialog(&root)?;
    Ok(())
}

fn welcome(main: &web_sys::Element) -> Result {
    let welcome = child(
        main,
        "section",
        &[("id", "welcome"), ("class", "welcome"), ("hidden", "")],
        None,
    )?;
    child(
        &welcome,
        "p",
        &[("class", "eyebrow")],
        Some("YOUR WORK, YOUR APP"),
    )?;
    let heading = child(&welcome, "h1", &[], Some("仕事に合うアプリを、"))?;
    child(&heading, "br", &[], None)?;
    append(&heading, &element("span", Some("ここから。"), None)?)?;
    let description = child(
        &welcome,
        "p",
        &[],
        Some("顧客リストも、在庫も、日々のタスクも。"),
    )?;
    child(&description, "br", &[], None)?;
    append(
        &description,
        &element(
            "span",
            Some("必要な項目を選んで、自分のアプリを作りましょう。"),
            None,
        )?,
    )?;
    child(
        &welcome,
        "button",
        &[("id", "first-app"), ("class", "primary large")],
        Some("最初のアプリを作る"),
    )?;
    let templates = child(&welcome, "div", &[("class", "templates")], None)?;
    for (id, title, description) in [
        ("customer-template", "顧客管理", "顧客名・連絡先・対応状況"),
        ("inventory-template", "在庫管理", "商品名・数量・メモ"),
    ] {
        let template = child(&templates, "button", &[("id", id)], None)?;
        child(&template, "strong", &[], Some(title))?;
        child(&template, "span", &[], Some(description))?;
    }
    Ok(())
}

fn workspace(main: &web_sys::Element) -> Result {
    let workspace = child(
        main,
        "section",
        &[("id", "app-workspace"), ("hidden", "")],
        None,
    )?;
    let heading = child(&workspace, "div", &[("class", "page-heading")], None)?;
    let title = child(&heading, "div", &[], None)?;
    child(&title, "p", &[("class", "eyebrow")], Some("WORKSPACE"))?;
    child(&title, "h1", &[("id", "app-title")], None)?;
    child(
        &title,
        "p",
        &[("id", "record-count"), ("class", "muted")],
        None,
    )?;
    let actions = child(&heading, "div", &[("class", "workspace-actions")], None)?;
    child(
        &actions,
        "button",
        &[("id", "edit-app")],
        Some("フォーム設定"),
    )?;
    child(
        &actions,
        "button",
        &[("id", "new-record"), ("class", "primary")],
        Some("＋ レコードを追加"),
    )?;
    let search = child(
        &workspace,
        "form",
        &[("id", "search-form"), ("class", "search")],
        None,
    )?;
    child(
        &search,
        "label",
        &[("for", "search")],
        Some("レコードを検索"),
    )?;
    let row = child(&search, "div", &[], None)?;
    child(
        &row,
        "input",
        &[
            ("id", "search"),
            ("type", "search"),
            ("maxlength", "128"),
            ("placeholder", "文字を入力して検索"),
        ],
        None,
    )?;
    child(&row, "button", &[("type", "submit")], Some("検索"))?;
    child(
        &row,
        "button",
        &[("id", "clear-search"), ("type", "button")],
        Some("クリア"),
    )?;
    let export = child(&workspace, "div", &[("class", "export-tools")], None)?;
    child(
        &export,
        "button",
        &[("id", "export-csv"), ("disabled", "")],
        Some("CSVを出力"),
    )?;
    child(
        &export,
        "span",
        &[("class", "muted")],
        Some("検索結果を全ページ出力（最大10,000件・16 MiB）"),
    )?;
    child(
        &workspace,
        "p",
        &[
            ("id", "export-status"),
            ("class", "status"),
            ("role", "status"),
            ("aria-live", "polite"),
        ],
        None,
    )?;
    child(
        &workspace,
        "p",
        &[("id", "table-hint"), ("class", "muted"), ("hidden", "")],
        Some("表を横にスクロールして、項目や編集ボタンを表示できます。"),
    )?;
    let scroll = child(&workspace, "div", &[("class", "table-scroll")], None)?;
    let table = child(&scroll, "table", &[], None)?;
    child(
        &table,
        "caption",
        &[("id", "table-caption"), ("class", "sr-only")],
        Some("レコード一覧"),
    )?;
    child(&table, "thead", &[("id", "table-head")], None)?;
    child(&table, "tbody", &[("id", "table-body")], None)?;
    let pagination = child(&workspace, "div", &[("class", "pagination")], None)?;
    child(
        &pagination,
        "button",
        &[("id", "previous-page")],
        Some("前のページ"),
    )?;
    child(&pagination, "span", &[("id", "page-number")], None)?;
    child(
        &pagination,
        "button",
        &[("id", "next-page")],
        Some("次のページ"),
    )?;
    Ok(())
}

fn app_dialog(root: &web_sys::Element) -> Result {
    let dialog = child(
        root,
        "dialog",
        &[
            ("id", "app-dialog"),
            ("aria-labelledby", "app-dialog-title"),
        ],
        None,
    )?;
    let form = child(&dialog, "form", &[("id", "app-form")], None)?;
    let heading = child(&form, "div", &[("class", "dialog-heading")], None)?;
    child(
        &heading,
        "h2",
        &[("id", "app-dialog-title")],
        Some("アプリを作る"),
    )?;
    child(
        &heading,
        "button",
        &[
            ("type", "button"),
            ("class", "close-app"),
            ("aria-label", "アプリ作成を閉じる"),
        ],
        Some("×"),
    )?;
    child(&form, "label", &[("for", "app-name")], Some("アプリ名"))?;
    child(
        &form,
        "input",
        &[
            ("id", "app-name"),
            ("required", ""),
            ("maxlength", "80"),
            ("placeholder", "例：顧客管理"),
        ],
        None,
    )?;
    child(
        &form,
        "p",
        &[
            ("id", "app-settings-hint"),
            ("class", "muted"),
            ("hidden", ""),
        ],
        Some("追加する項目は任意入力です。既存項目の種類・必須設定・選択肢は変更できません。"),
    )?;
    let section_heading = child(&form, "div", &[("class", "section-heading")], None)?;
    child(&section_heading, "h3", &[], Some("項目"))?;
    child(
        &section_heading,
        "span",
        &[("class", "muted")],
        Some("最大32項目"),
    )?;
    child(&form, "div", &[("id", "field-builder")], None)?;
    child(
        &form,
        "button",
        &[("id", "add-field"), ("type", "button")],
        Some("＋ 項目を追加"),
    )?;
    child(
        &form,
        "p",
        &[("id", "app-error"), ("class", "error"), ("role", "alert")],
        None,
    )?;
    child(
        &form,
        "button",
        &[("id", "reload-app"), ("type", "button"), ("hidden", "")],
        Some("最新版を読み込む（入力内容を置き換えます）"),
    )?;
    let actions = child(&form, "div", &[("class", "dialog-actions")], None)?;
    child(
        &actions,
        "button",
        &[("type", "button"), ("class", "close-app")],
        Some("キャンセル"),
    )?;
    child(
        &actions,
        "button",
        &[("id", "save-app"), ("type", "submit"), ("class", "primary")],
        Some("アプリを作成"),
    )?;
    Ok(())
}

fn record_dialog(root: &web_sys::Element) -> Result {
    let dialog = child(
        root,
        "dialog",
        &[
            ("id", "record-dialog"),
            ("aria-labelledby", "record-dialog-title"),
        ],
        None,
    )?;
    let form = child(&dialog, "form", &[("id", "record-form")], None)?;
    let heading = child(&form, "div", &[("class", "dialog-heading")], None)?;
    child(
        &heading,
        "h2",
        &[("id", "record-dialog-title")],
        Some("レコードを追加"),
    )?;
    child(
        &heading,
        "button",
        &[
            ("type", "button"),
            ("class", "close-record"),
            ("aria-label", "レコード編集を閉じる"),
        ],
        Some("×"),
    )?;
    child(&form, "div", &[("id", "record-fields")], None)?;
    child(
        &form,
        "p",
        &[
            ("id", "record-error"),
            ("class", "error"),
            ("role", "alert"),
        ],
        None,
    )?;
    child(
        &form,
        "button",
        &[("id", "reload-record"), ("type", "button"), ("hidden", "")],
        Some("最新版を読み込む（入力内容を置き換えます）"),
    )?;
    let actions = child(&form, "div", &[("class", "dialog-actions")], None)?;
    child(
        &actions,
        "button",
        &[
            ("id", "delete-record"),
            ("type", "button"),
            ("class", "danger"),
            ("hidden", ""),
        ],
        Some("削除"),
    )?;
    child(&actions, "span", &[("class", "spacer")], None)?;
    child(
        &actions,
        "button",
        &[("type", "button"), ("class", "close-record")],
        Some("キャンセル"),
    )?;
    child(
        &actions,
        "button",
        &[
            ("id", "save-record"),
            ("type", "submit"),
            ("class", "primary"),
        ],
        Some("保存"),
    )?;
    Ok(())
}
