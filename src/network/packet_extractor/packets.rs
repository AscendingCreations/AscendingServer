use crate::{gametypes::*, network::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientPacket {
    OnlineCheck,
    Login(String),
    HandShake,
    Move {
        dir: u8,
        pos: Position,
    },
    Dir(u8),
    Attack {
        dir: u8,
        target_id: Option<u64>,
        target_map: Option<MapPosition>,
    },
    UseItem(u16),
    Unequip(u16),
    SwitchInvSlot {
        oldslot: u16,
        newslot: u16,
        amount: u16,
    },
    PickUp,
    DropItem {
        slot: u16,
        amount: u16,
    },
    DeleteItem(u16),
    SwitchStorageSlot {
        oldslot: u16,
        newslot: u16,
        amount: u16,
    },
    DeleteStorageItem(u16),
    DepositItem {
        inv_slot: u16,
        bank_slot: u16,
        amount: u16,
    },
    WithdrawItem {
        inv_slot: u16,
        bank_slot: u16,
        amount: u16,
    },
    Message {
        channel: MessageChannel,
        msg: String,
        name: String,
    },
    Command(Command),
    SetTarget {
        target_id: Option<u64>,
        target_map: Option<MapPosition>,
    },
    CloseStorage,
    CloseShop,
    CloseTrade,
    BuyItem {
        slot: u16,
    },
    SellItem {
        slot: u16,
        amount: u16,
    },
    AddTradeItem {
        slot: u16,
        amount: u16,
    },
    RemoveTradeItem {
        slot: u16,
        amount: u16,
    },
    UpdateTradeMoney {
        amount: u64,
    },
    SubmitTrade,
    AcceptTrade,
    DeclineTrade,
    Ping,
    Disconnect,
}

pub fn handle_login(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let token = data.read::<String>()?;

    Ok(ClientPacket::Login(token))
}

pub fn handle_move(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let dir = data.read::<u8>()?;
    let pos = data.read::<Position>()?;

    Ok(ClientPacket::Move { dir, pos })
}

pub fn handle_dir(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let dir = data.read::<u8>()?;

    Ok(ClientPacket::Dir(dir))
}

pub fn handle_attack(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let dir = data.read::<u8>()?;
    let target_id = data.read::<Option<u64>>()?;
    let target_map = data.read::<Option<MapPosition>>()?;

    Ok(ClientPacket::Attack {
        dir,
        target_id,
        target_map,
    })
}

pub fn handle_useitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;

    Ok(ClientPacket::UseItem(slot))
}

pub fn handle_unequip(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;

    Ok(ClientPacket::Unequip(slot))
}

pub fn handle_switchinvslot(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let oldslot = data.read::<u16>()?;
    let newslot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::SwitchInvSlot {
        oldslot,
        newslot,
        amount,
    })
}

pub fn handle_dropitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::DropItem { slot, amount })
}

pub fn handle_deleteitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;

    Ok(ClientPacket::DeleteItem(slot))
}

pub fn handle_switchstorageslot(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let oldslot = data.read::<u16>()?;
    let newslot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::SwitchStorageSlot {
        oldslot,
        newslot,
        amount,
    })
}

pub fn handle_deletestorageitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;

    Ok(ClientPacket::DeleteStorageItem(slot))
}

pub fn handle_deposititem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let inv_slot = data.read::<u16>()?;
    let bank_slot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::DepositItem {
        inv_slot,
        bank_slot,
        amount,
    })
}

pub fn handle_withdrawitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let inv_slot = data.read::<u16>()?;
    let bank_slot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::WithdrawItem {
        inv_slot,
        bank_slot,
        amount,
    })
}

pub fn handle_message(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let channel: MessageChannel = data.read()?;
    let msg = data.read::<String>()?;
    let name = data.read::<String>()?;

    Ok(ClientPacket::Message { channel, msg, name })
}

pub fn handle_command(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let command = data.read::<Command>()?;

    Ok(ClientPacket::Command(command))
}

pub fn handle_settarget(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let target_id = data.read::<Option<u64>>()?;
    let target_map = data.read::<Option<MapPosition>>()?;

    Ok(ClientPacket::SetTarget {
        target_id,
        target_map,
    })
}

pub fn handle_buy_item(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;

    Ok(ClientPacket::BuyItem { slot })
}

pub fn handle_sellitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::SellItem { slot, amount })
}

pub fn handle_addtradeitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::AddTradeItem { slot, amount })
}

pub fn handle_removetradeitem(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let slot = data.read::<u16>()?;
    let amount = data.read::<u16>()?;

    Ok(ClientPacket::RemoveTradeItem { slot, amount })
}

pub fn handle_updatetrademoney(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let amount = data.read::<u64>()?;

    Ok(ClientPacket::UpdateTradeMoney { amount })
}
