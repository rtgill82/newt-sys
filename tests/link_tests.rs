/// These tests should compile cleanly without type errors
extern crate newt_sys;
use std::ffi::CString;
use std::os::raw::{c_char,c_void};
use std::ptr::null_mut;
use newt_sys::*;

#[test]
fn form_add_hotkey_type() {
    unsafe {
        let form = newtForm(null_mut(), null_mut(), 0);
        newtFormAddHotKey(form, NEWT_KEY_TAB);
        newtFormAddHotKey(form, NEWT_KEY_ENTER);
        newtFormAddHotKey(form, NEWT_KEY_SUSPEND);
        newtFormAddHotKey(form, NEWT_KEY_ESCAPE);
        newtFormAddHotKey(form, NEWT_KEY_RETURN);
    }
}

#[test]
fn checkbox_tree_entry_value_types() {
    unsafe {
        let tree = newtCheckboxTree(0, 0, 0, 0);
        add_item(tree, "Tree1", 1, &[0, NEWT_ARG_LAST]);
        add_item(tree, "Item2", 2, &[0, NEWT_ARG_APPEND, NEWT_ARG_LAST]);
        add_item(tree, "Item3", 3, &[0, NEWT_ARG_APPEND, NEWT_ARG_LAST]);

        let value = get_entry_value(tree, 1);
        assert!(value == NEWT_CHECKBOXTREE_COLLAPSED);

        // newtCheckboxTreeSetEntryValue returns immediately if the item being
        // set is a branch, but the following validates that
        // NEWT_CHECKBOXTREE_EXPANDED is of the type expected by both
        // newtCheckboxTreeGetEntryValue and newtCheckboxTreeSetEntryValue.
        set_entry_value(tree, 1, NEWT_CHECKBOXTREE_EXPANDED);

        set_entry_value(tree, 2, NEWT_CHECKBOXTREE_SELECTED); 
        let value: char = get_entry_value(tree, 2) as u8 as char;
        assert!(value == '*');

        set_entry_value(tree, 2, NEWT_CHECKBOXTREE_UNSELECTED);
        let value: char = get_entry_value(tree, 2) as u8 as char;
        assert!(value == ' ');
    }
}

fn add_item(tree: newtComponent, text: &str, data: i32, indexes: &[i32]) {
    unsafe {
        let c_str = CString::new(text).unwrap().as_ptr();
        let c_data: *mut c_void = data as *mut c_void;
        let c_ary: *mut i32 = indexes.as_ptr() as *mut i32;
        newtCheckboxTreeAddArray(tree, c_str, c_data, 0, c_ary);
    }
}

fn set_entry_value(tree: newtComponent, data: i32, value: c_char) {
    unsafe {
        let c_data: *mut c_void = data as *mut c_void;
        newtCheckboxTreeSetEntryValue(tree, c_data, value);
    }
}

fn get_entry_value(tree: newtComponent, data: i32) -> c_char {
    unsafe {
        let c_data: *mut c_void = data as *mut c_void;
        newtCheckboxTreeGetEntryValue(tree, c_data)
    }
}
