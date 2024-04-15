use bytey::{ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ByteBufferRead, ByteBufferWrite,
)]
#[repr(u32)]
pub enum ServerPackets {
    Ping,
    Status,
    AlertMsg,
    FltAlert,
    HandShake,
    LoginOk,
    Ingame,
    UpdateMap,
    MapItems,
    MyIndex,
    PlayerData,
    PlayerSpawn,
    PlayerMove,
    PlayerWarp,
    PlayerMapSwap,
    Dataremovelist,
    Dataremove,
    PlayerDir,
    PlayerVitals,
    PlayerInv,
    PlayerInvSlot,
    PlayerStorage,
    PlayerStorageSlot,
    KeyInput,
    PlayerAttack,
    PlayerEquipment,
    PlayerAction,
    PlayerLevel,
    PlayerMoney,
    PlayerStun,
    PlayerVariables,
    PlayerVariable,
    PlayerDeath,
    NpcDeath,
    PlayerPvp,
    PlayerPk,
    PlayerEmail,
    NpcData,
    NpcMove,
    NpcWarp,
    NpcDir,
    NpcVital,
    NpcAttack,
    NpcStun,
    ChatMsg,
    Sound,
    Target,
    SyncCheck,
    EntityUnload,
    LoadStatus,
    OpenStorage,
    OpenShop,
    ClearIsUsingType,
    UpdateTradeItem,
    UpdateTradeMoney,
    InitTrade,
    TradeStatus,
    TradeRequest,
    PlayItemSfx,
    FloatTextDamage,
    FloatTextHeal,
    ServerPacketCount,
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ByteBufferRead, ByteBufferWrite, Hash,
)]
pub enum ClientPacket {
    Ping,
    Register,
    Login,
    HandShake,
    Move,
    Dir,
    Attack,
    UseItem,
    Unequip,
    SwitchInvSlot,
    PickUp,
    DropItem,
    DeleteItem,
    SwitchStorageSlot,
    DeleteStorageItem,
    DepositItem,
    WithdrawItem,
    Message,
    Command,
    SetTarget,
    CloseStorage,
    CloseShop,
    CloseTrade,
    BuyItem,
    SellItem,
    AddTradeItem,
    RemoveTradeItem,
    UpdateTradeMoney,
    SubmitTrade,
    AcceptTrade,
    DeclineTrade,
    Size,
}
