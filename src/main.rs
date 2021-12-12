#![allow(incomplete_features)]
#![feature(backtrace, capture_disjoint_fields)]


use arcstr::ArcStr;
use env_logger::TimestampPrecision;
use futures_util::FutureExt;
use log::info;
use mesagisto_client::{EitherExt, MesagistoConfig};
use mesagisto_client::data::{Packet, message};
use mesagisto_client::data::message::{MessageType, Profile};
use mesagisto_client::server::SERVER;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

  std::env::set_var("RUST_BACKTRACE", "1");
  std::backtrace::Backtrace::force_capture();
  env_logger::builder()
    .write_style(env_logger::WriteStyle::Auto)
    .filter(None, log::LevelFilter::Error)
    .format_timestamp(None)
    .filter(Some("mesagisto_diagnose"), log::LevelFilter::Trace)
    .filter(Some("mesagisto_client"), log::LevelFilter::Trace)
    .init();


  let yes = vec!["yes","y"];

  info!("信使诊断工具启动中...");
  info!("注: 有默认项时可输入Enter使用默认项.");
  let mut line = String::new();
  let cipher_enable:bool;
  let cipher_key:String;
  let refuse_plain:bool;
  info!("是否启用加密 yes/no or y/n，默认yes");
  next_line(&mut line).await?;
  if yes.contains(&&*line.to_lowercase()) || line.to_lowercase() == "" {
    cipher_enable = true;
  } else {
    cipher_enable = false;
  }
  if cipher_enable {
    info!("请输入加密密钥");
    next_line(&mut line).await?;
    cipher_key = line.to_string();
    info!("是否拒绝非加密消息 yes/no or y/n, 默认yes");
    next_line(&mut line).await?;
    if yes.contains(&&*line.to_lowercase()) || line.to_lowercase() == "" {
      refuse_plain = true;
    } else {
      refuse_plain = false;
    }
  } else {
    cipher_key = String::new();
    refuse_plain = false;
  }
  info!("请输入服务器地址, 默认 nats://itsusinn.site:4222");

  let server_addr:String;
  next_line(&mut line).await?;
  if line.to_lowercase() == "" {
    server_addr = "nats://itsusinn.site:4222".to_string();
  } else {
    server_addr = line.trim().to_string();
  }
  MesagistoConfig::builder()
    .name("diagnose")
    .cipher_enable(cipher_enable)
    .cipher_key(cipher_key)
    .cipher_refuse_plain(refuse_plain)
    .proxy(None)
    .nats_address(server_addr)
    .photo_url_resolver(|_| {
      async {
        anyhow::Result::Ok(ArcStr::new())
      }.boxed()
    })
    .build()
    .apply()
    .await;
  info!("信使诊断工具启动完成");
  info!("请输入频道地址");
  next_line(&mut line).await?;
  let channel_addr = ArcStr::from(line.clone());
  dbg!(channel_addr.clone());

  let profile = Profile {
    id: 0i64.to_be_bytes().into(),
    username: Some("mesagisto-diagnose".into()),
    nick: None,
  };
  let mut chain = Vec::<MessageType>::new();
  chain.push(MessageType::Text {
    content: "诊断工具已连接到该频道".to_string(),
  });
  let message = message::Message {
    profile,
    id: 0i64.to_be_bytes().to_vec(),
    chain,
    reply: None,
  };
  let packet = Packet::from(message.tl())?;
  SERVER.send_and_receive(0,channel_addr.clone(), packet, receive_from_server).await?;

  loop {
    next_line(&mut line).await?;
    let profile = Profile {
      id: 0i64.to_be_bytes().into(),
      username: Some("mesagisto-diagnose".into()),
      nick: None,
    };
    let mut chain = Vec::<MessageType>::new();
    chain.push(MessageType::Text {
      content: line.to_string(),
    });
    let message = message::Message {
      profile,
      id: 0i64.to_be_bytes().to_vec(),
      chain,
      reply: None,
    };
    let packet = Packet::from(message.tl())?;
    info!("发送消息: {}", line);
    SERVER.send_and_receive(0,channel_addr.clone(), packet, receive_from_server).await?;
  }
}

async fn next_line(buf: &mut String) -> async_std::io::Result<usize> {
  buf.clear();
  let r = async_std::io::stdin().read_line(buf).await;
  buf.remove(buf.len() - 1);
  r
}
pub async fn receive_from_server(message: nats::asynk::Message, _: i64) -> anyhow::Result<()> {
  let packet = Packet::from_cbor(&message.data)?;
  match packet {
    either::Left(msg) => {
      info!("收到消息：");
      dbg!(msg);
    }
    either::Right(_) => {
      info!("收到事件");
      // dbg!(event);
    }
  }
  Ok(())

}