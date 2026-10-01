//! `icon-matches-product` (section 13.10): an Item's subtitle names a product, and its icon
//! must be that product's own icon, or a category icon when the product has none.

use crate::checks::{CheckName, CheckReport, Defect, is_word_character};
use crate::document::{IconName, Node, Page};
use crate::grammar::{
    GRAMMAR_PRODUCT_NAMES_MAX, GRAMMAR_PRODUCTS_MAX, Grammar, IconClass, IconProducts,
};
use crate::walk::{NodeRef, body_nodes};

/// Why icon-matches-product does not apply to a page whose items have no icon table.
const NO_ICON_TABLE: &str = "grammar has no icon table";

/// The byte offset of the first occurrence of `name` in `text`, compared ASCII
/// case-insensitively, with no ASCII alphanumeric or `_` directly before or after it.
fn find_at_word_boundary(text: &str, name: &str) -> Option<usize> {
    if name.is_empty() {
        return None;
    }
    text.char_indices().find_map(|(start, _)| {
        let end = start.checked_add(name.len())?;
        let candidate = text.get(start..end)?;
        if !candidate.eq_ignore_ascii_case(name) {
            return None;
        }
        let before = text.get(..start).and_then(|head| head.chars().next_back());
        let after = text.get(end..).and_then(|tail| tail.chars().next());
        let bounded =
            !before.is_some_and(is_word_character) && !after.is_some_and(is_word_character);
        bounded.then_some(start)
    })
}

/// The product a subtitle names: every name of the table found in `subtitle`, ASCII
/// case-insensitive and at word boundaries, and of those the longest; on a tie, the one that
/// starts first. None when the subtitle names nothing in the table.
pub fn named_product<'a>(table: &'a [IconProducts], subtitle: &str) -> Option<&'a str> {
    let mut best: Option<(&'a str, usize)> = None;
    for row in table.iter().take(GRAMMAR_PRODUCTS_MAX) {
        for name in row.names.iter().take(GRAMMAR_PRODUCT_NAMES_MAX) {
            let Some(start) = find_at_word_boundary(subtitle, name) else {
                continue;
            };
            let better = match best {
                None => true,
                Some((best_name, best_start)) => {
                    name.len() > best_name.len()
                        || (name.len() == best_name.len() && start < best_start)
                }
            };
            if better {
                best = Some((name.as_str(), start));
            }
        }
    }
    best.map(|(name, _)| name)
}

/// The `product`-class icons whose row lists `product`, sorted by name.
fn product_icons(table: &[IconProducts], product: &str) -> Vec<IconName> {
    let mut icons: Vec<IconName> = table
        .iter()
        .take(GRAMMAR_PRODUCTS_MAX)
        .filter(|row| row.class == IconClass::Product)
        .filter(|row| {
            row.names
                .iter()
                .take(GRAMMAR_PRODUCT_NAMES_MAX)
                .any(|name| name == product)
        })
        .map(|row| row.icon)
        .collect();
    icons.sort_by_key(|icon| icon.as_str());
    icons.dedup();
    icons
}

/// True when the table lists `icon` with class `product`.
fn is_product_icon(table: &[IconProducts], icon: IconName) -> bool {
    table
        .iter()
        .take(GRAMMAR_PRODUCTS_MAX)
        .any(|row| row.icon == icon && row.class == IconClass::Product)
}

/// The defect message for an item whose subtitle names `product` and which carries `icon`,
/// or None when the icon fits the product.
fn mismatch(table: &[IconProducts], product: &str, icon: IconName) -> Option<String> {
    let own_icons = product_icons(table, product);
    if own_icons.is_empty() {
        is_product_icon(table, icon).then(|| {
            format!(
                "subtitle names {product}, which has no product icon; the item carries the product icon {}",
                icon.as_str()
            )
        })
    } else if own_icons.contains(&icon) {
        None
    } else {
        let names: Vec<&str> = own_icons.iter().map(|own| own.as_str()).collect();
        Some(format!(
            "subtitle names {product}, whose icon is {}; the item carries {}",
            names.join(" or "),
            icon.as_str()
        ))
    }
}

/// Each Item whose kind has a non-empty products table and that carries an `icon` or a
/// `subtitle` is examined. It is a defect when the subtitle names a product with product
/// icons and the item carries another icon, or names a product with no product icon and the
/// item carries a product icon. Not applicable when no item of the page has a kind with a
/// products table.
pub fn icon_matches_product(page: &Page, grammar: &Grammar) -> CheckReport {
    let mut has_table = false;
    let mut examined: u64 = 0;
    let mut defects = Vec::new();
    for entry in body_nodes(page) {
        let NodeRef::Node(Node::Item(item)) = entry.node else {
            continue;
        };
        let Some(table) = grammar
            .item(&item.kind)
            .map(|kind| kind.products.as_slice())
            .filter(|table| !table.is_empty())
        else {
            continue;
        };
        has_table = true;
        if item.icon.is_none() && item.subtitle.is_none() {
            continue;
        }
        examined += 1;
        let product = item
            .subtitle
            .as_deref()
            .and_then(|subtitle| named_product(table, subtitle));
        if let (Some(product), Some(icon)) = (product, item.icon)
            && let Some(message) = mismatch(table, product, icon)
        {
            defects.push(Defect {
                pointer: entry.pointer,
                message,
            });
        }
    }
    if !has_table {
        return CheckReport::not_applicable(CheckName::IconMatchesProduct, NO_ICON_TABLE);
    }
    CheckReport {
        check: CheckName::IconMatchesProduct,
        examined,
        defects,
        not_applicable: None,
    }
}
