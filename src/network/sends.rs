use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    network::*,
    tasks::*,
    GlobalKey,
};
use std::ops::Range;

impl MapActorStore {
    #[inline]
    pub async fn send_infomsg(
        &mut self,
        key: GlobalKey,
        message: String,
        close_socket: u8,
    ) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::AlertMsg)?;
        buf.write(message)?;
        buf.write(close_socket)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_fltalert(
        &mut self,
        key: GlobalKey,
        message: String,
        ftltype: FtlType,
    ) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::FltAlert)?;
        buf.write(ftltype)?;
        buf.write(message)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_loginok(&mut self, key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::LoginOk)?;
        buf.write(self.time.hour)?;
        buf.write(self.time.min)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_myindex(&mut self, key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::MyIndex)?;
        buf.write(key)?;
        buf.write(key)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    pub async fn send_move_ok(&mut self, key: GlobalKey, move_ok: bool) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::MoveOk)?;
        buf.write(move_ok)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_playerdata(&mut self, key: GlobalKey) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerData)?;
            buf.write(&player.username)?;
            buf.write(player.access)?;
            buf.write(player.dir)?;
            buf.write(&player.equipment)?;
            buf.write(player.hidden)?;
            buf.write(player.level)?;
            buf.write(player.death)?;
            buf.write(player.damage)?;
            buf.write(player.defense)?;
            buf.write(player.position)?;
            buf.write(player.pk)?;
            buf.write(player.pvpon)?;
            buf.write(player.sprite as u8)?;
            buf.write(player.vital)?;
            buf.write(player.vitalmax)?;
            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    pub async fn send_ping(&mut self, key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::OnlineCheck)?;
        buf.write(0u64)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_inv(&mut self, key: GlobalKey) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerInv)?;
            buf.write(&player.inventory)?;

            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_invslot(&mut self, key: GlobalKey, id: usize) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerInvSlot)?;
            buf.write(id)?;
            buf.write(player.inventory[id])?;
            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_store(&mut self, key: GlobalKey, range: Range<usize>) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerStorage)?;
            buf.write(range.clone())?;
            buf.write(&player.storage[range])?;
            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_storeslot(&mut self, key: GlobalKey, id: usize) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerStorageSlot)?;
            buf.write(id)?;
            buf.write(player.storage[id])?;
            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_equipment(&mut self, map: &mut MapActor, key: GlobalKey) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerEquipment)?;
            buf.write(key)?;
            buf.write(&player.equipment)?;
            buf.finish()?;

            return map.send_to_maps(self, buf, None).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_level(&mut self, key: GlobalKey) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerLevel)?;
            buf.write(player.level)?;
            buf.write(player.levelexp)?;
            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_money(&mut self, key: GlobalKey) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPacketID::PlayerMoney)?;
            buf.write(player.vals)?;
            buf.finish()?;

            return self.send_to(key, buf).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_pk(
        &mut self,
        map: &mut MapActor,
        key: GlobalKey,
        toself: bool,
    ) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut buf = MByteBuffer::new_packet()?;
            let closure = |toself, id| if toself { Some(id) } else { None };

            buf.write(ServerPacketID::PlayerPk)?;
            buf.write(player.pk)?;
            buf.finish()?;

            return map.send_to_maps(self, buf, closure(toself, key)).await;
        }

        Ok(())
    }

    #[inline]
    pub async fn send_message(
        &mut self,
        map: &mut MapActor,
        key: GlobalKey,
        msg: String,
        head: String,
        chan: MessageChannel,
        id: Option<GlobalKey>,
    ) -> Result<()> {
        let access = match self.players.get(&key) {
            Some(p) => p.access,
            None => UserAccess::None,
        };

        match chan {
            MessageChannel::Map => DataTaskToken::MapChat
                .add_task(map, message_packet(chan, head, msg, Some(access))?)?,
            MessageChannel::Global => DataTaskToken::GlobalChat
                .add_task(map, message_packet(chan, head, msg, Some(access))?)?,
            MessageChannel::Party | MessageChannel::Trade | MessageChannel::Help => {}
            MessageChannel::Private => {
                let mut buf = MByteBuffer::new_packet()?;
                buf.write(ServerPacketID::ChatMsg)?;
                buf.write(1_u32)?;
                buf.write(chan)?;
                buf.write(head)?;
                buf.write(msg)?;
                buf.write(Some(access))?;
                buf.finish()?;

                if let Some(i) = id {
                    self.send_to(i, buf.clone()).await?;
                }

                self.send_to(key, buf).await?;
            }
            MessageChannel::Guild => {}
            MessageChannel::Quest | MessageChannel::Npc => {
                let mut buf = MByteBuffer::new_packet()?;

                buf.write(ServerPacketID::ChatMsg)?;
                buf.write(1_u32)?;
                buf.write(chan)?;
                buf.write(head)?;
                buf.write(msg)?;
                buf.write(Some(access))?;
                buf.finish()?;

                self.send_to(key, buf).await?;
            }
        }

        Ok(())
    }

    #[inline]
    pub async fn send_openstore(&mut self, key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::OpenStorage)?;
        buf.write(1_u32)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_openshop(&mut self, key: GlobalKey, shop_index: u16) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::OpenShop)?;
        buf.write(shop_index)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_clearisusingtype(&mut self, key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::ClearIsUsingType)?;
        buf.write(1_u16)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_updatetradeitem(
        &mut self,
        target_key: GlobalKey,
        send_key: GlobalKey,
        trade_slot: u16,
    ) -> Result<()> {
        let buf = if let Some(player) = self.players.get(&target_key) {
            if let Some(trade) = &player.trade {
                let mut buf = MByteBuffer::new_packet()?;

                buf.write(ServerPacketID::UpdateTradeItem)?;
                buf.write(target_key == send_key)?;
                buf.write(trade_slot)?;
                buf.write(trade.items[trade_slot as usize])?;
                buf.finish()?;

                Some(buf)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(buf) = buf {
            self.send_to(send_key, buf).await
        } else {
            Ok(())
        }
    }

    #[inline]
    pub async fn send_updatetrademoney(
        &mut self,
        target_key: GlobalKey,
        send_key: GlobalKey,
    ) -> Result<()> {
        let buf = if let Some(player) = self.players.get(&target_key) {
            if let Some(trade) = &player.trade {
                let mut buf = MByteBuffer::new_packet()?;

                buf.write(ServerPacketID::UpdateTradeMoney)?;
                buf.write(trade.vals)?;
                buf.finish()?;
                Some(buf)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(buf) = buf {
            self.send_to(send_key, buf).await
        } else {
            Ok(())
        }
    }

    #[inline]
    pub async fn send_inittrade(&mut self, key: GlobalKey, target_key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::InitTrade)?;
        buf.write(target_key)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_tradestatus(
        &mut self,
        key: GlobalKey,
        my_status: &TradeStatus,
        their_status: &TradeStatus,
    ) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::TradeStatus)?;
        buf.write(*my_status)?;
        buf.write(*their_status)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_traderequest(&mut self, key: GlobalKey, target_key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::TradeRequest)?;
        buf.write(key)?;
        buf.finish()?;

        self.send_to(target_key, buf).await
    }

    #[inline]
    pub async fn send_playitemsfx(&mut self, key: GlobalKey, item_index: u16) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::PlayItemSfx)?;
        buf.write(item_index)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }

    #[inline]
    pub async fn send_gameping(&mut self, key: GlobalKey) -> Result<()> {
        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPacketID::Ping)?;
        buf.write(0u64)?;
        buf.finish()?;

        self.send_to(key, buf).await
    }
}
