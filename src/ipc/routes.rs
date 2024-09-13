use crate::{gametypes::*, ipc::*, network::ByteBufferExt};
use bytey::ByteBuffer;
use interprocess::local_socket::tokio::SendHalf;
use mmap_bytey::MByteBuffer;
use std::backtrace::Backtrace;
use tokio::{io::AsyncWriteExt, sync::oneshot};

pub fn new_cache(packet_id: ServerIPCID) -> Result<ByteBuffer> {
    //Set it to the max packet size - the size holder - packet_id - count
    let mut buffer = ByteBuffer::new_packet()?;
    //Write the packet ID so we know where it goes.
    buffer.write(packet_id)?;
    //preallocate space for count.
    buffer.write(0u32)?;
    Ok(buffer)
}

pub fn finish_cache(buffer: &mut ByteBuffer, count: u32, is_finished: bool) -> Result<()> {
    if !is_finished {
        //Move it 8 bytes for Size + 2 bytes for Packet ID enum to get count location.
        buffer.move_cursor(10)?;
        //Write the count from the offset cursor position.
        //This will overwrite old data which in this case is empty.
        buffer.write(count)?;
        //finish the buffer off. This sets the Packet size and makes sure the cursor is
        //back to zero again.
        buffer.finish()?;
    }
    Ok(())
}

pub async fn handle_get_users(
    store: &mut IPCActor,
    _data: &mut MByteBuffer,
    tx: &mut SendHalf,
) -> Result<()> {
    let (one_tx, one_rx) = oneshot::channel::<InfoOutGoing>();

    if let Err(e) = store
        .info_tx
        .send(InfoIncomming::GetOnlineUsers(one_tx))
        .await
    {
        return Err(AscendingError::TokioMPSCInfoSendError {
            error: Box::new(e),
            backtrace: Box::new(Backtrace::capture()),
        });
    }

    if let InfoOutGoing::SendOnlineUsers(users) = one_rx.await? {
        let mut buffer = new_cache(ServerIPCID::UserList)?;
        let mut count = 0;

        for user in users {
            count += 1;

            buffer.write(user)?;
        }

        finish_cache(&mut buffer, count, false)?;

        tx.write_all(buffer.as_slice()).await?;
    }

    Ok(())
}
