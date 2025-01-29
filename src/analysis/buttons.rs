use log::debug;

use memflow::prelude::v1::*;

use serde::{Deserialize, Serialize};

use skidscan_macros::signature;

use crate::error::{Error, Result};
use crate::source2::KeyButton;

#[derive(Deserialize, Serialize)]
pub struct Button {
    pub name: String,
    pub value: u32,
}

pub fn buttons(process: &mut IntoProcessInstanceArcBox<'_>) -> Result<Vec<Button>> {
    let module = process.module_by_name("libclient.so")?;
    let buf = process.read_raw(module.base, module.size as _)?;

    let list_addr = signature!("48 8D 15 ? ? ? ? 66 44 89 ? ? 48 8D 35")
        .scan(&buf)
        .and_then(|result| process.read_addr64_rip(module.base + result).ok())
        .ok_or(Error::Other("unable to read button list address"))?;

    read_buttons(process, &module, list_addr)
}

fn read_buttons(
    process: &mut IntoProcessInstanceArcBox<'_>,
    module: &ModuleInfo,
    list_addr: Address,
) -> Result<Vec<Button>> {
    let mut buttons = Vec::new();

    let mut cur_button = Pointer64::<KeyButton>::from(process.read_addr64(list_addr)?);

    while !cur_button.is_null() {
        let button = cur_button.read(process)?;
        let name = button.name.read_string(process)?.to_string();

        let value =
            ((cur_button.address() - module.base) + offset_of!(KeyButton.state) as i64) as u32;

        debug!(
            "found button: {} at {:#X} ({} + {:#X})",
            name,
            value as u64 + module.base.to_umem(),
            module.name,
            value
        );

        buttons.push(Button { name, value });

        cur_button = button.next;
    }

    buttons.sort_unstable_by(|a, b| a.name.cmp(&b.name));

    Ok(buttons)
}
