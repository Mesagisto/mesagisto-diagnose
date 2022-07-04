#![allow(incomplete_features)]
#![feature(backtrace, capture_disjoint_fields)]

use arcstr::ArcStr;
use futures_util::FutureExt;
use mesagisto_client::data::message::{MessageType, Profile};
use mesagisto_client::data::{message, Packet};
use mesagisto_client::server::SERVER;
use mesagisto_client::{EitherExt, MesagistoConfig};
use tokio::io::{BufReader, AsyncBufReadExt};
use tracing::{info, Level};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
  std::env::set_var("RUST_BACKTRACE", "full");
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::OffsetTime::new(
          // use local time
          time::UtcOffset::__from_hms_unchecked(8, 0, 0),
          time::macros::format_description!(
            "[year repr:last_two]-[month]-[day] [hour]:[minute]:[second]"
          ),
        )),
    )
    .with(
      tracing_subscriber::filter::Targets::new()
        .with_target("mesagisto_diagnose", Level::TRACE)
        .with_target("mesagisto_client", Level::TRACE)
        .with_default(Level::WARN),
    )
    .init();
  run().await.unwrap();
}

async fn run() -> anyhow::Result<()> {
  info!("信使诊断工具启动中...");
  info!("注: 有默认项时可按下Enter使用默认项.");
  let mut line = String::new();

  info!("请输入加密密钥,默认 test");
  next_line(&mut line).await?;
  let cipher_key = if line.to_lowercase() == "" {
    "test".to_string()
  } else {
    line.trim().to_string()
  };
  info!("请输入服务器地址, 默认 nats://nats.mesagisto.org:4222");

  next_line(&mut line).await?;
  let server_addr = if line.to_lowercase() == "" {
    "nats://nats.mesagisto.org:4222".to_string()
  } else {
    line.trim().to_string()
  };
  MesagistoConfig::builder()
    .name("diagnose")
    .cipher_key(cipher_key)
    .proxy(None)
    .nats_address(server_addr)
    .photo_url_resolver(|_| async { anyhow::Result::Ok(ArcStr::new()) }.boxed())
    .build()
    .apply()
    .await?;
  info!("信使诊断工具启动完成");
  info!("请输入频道地址, 默认 test");
  next_line(&mut line).await?;
  let channel_addr:ArcStr = if line.to_lowercase() == "" {
    "test".into()
  } else {
    line.trim().into()
  };
  let channel_addr_clone = channel_addr.clone();
  tokio::task::spawn(async move {
    SERVER
      .recv("".into(), &channel_addr_clone, server_msg_handler)
      .await
      .unwrap()
  });

  let profile = Profile {
    id: 0i64.to_be_bytes().into(),
    username: Some("mesagisto-diagnose".into()),
    nick: None,
  };
  let chain = vec![MessageType::Text {
    content: "诊断工具已连接到该频道".to_string(),
  }];
  let message = message::Message {
    profile,
    id: 0i64.to_be_bytes().to_vec(),
    chain,
    reply: None,
  };
  let packet = Packet::from(message.tl())?;
  SERVER.send(&"".into(), &channel_addr, packet, None).await?;

  loop {
    next_line(&mut line).await?;
    let profile = Profile {
      id: 0i64.to_be_bytes().into(),
      username: Some("mesagisto-diagnose".into()),
      nick: None,
    };
    let chain = vec![MessageType::Text {
      content: line.to_string(),
    }];
    let message = message::Message {
      profile,
      id: 0i64.to_be_bytes().to_vec(),
      chain,
      reply: None,
    };
    let packet = Packet::from(message.tl())?;
    info!("发送消息: {}", line);
    SERVER.send(&"".into(), &channel_addr, packet, None).await?;
  }
}

async fn next_line(buf: &mut String) -> tokio::io::Result<usize> {
  buf.clear();
  let mut stdin = BufReader::new(tokio::io::stdin());
  let res = stdin.read_line(buf).await?;
  buf.remove(buf.len() - 1);
  Ok(res)
}
pub async fn server_msg_handler(
  message: nats::Message,
  _: ArcStr,
) -> anyhow::Result<()> {
  let packet = Packet::from_cbor(&message.payload);
  let packet = match packet {
    Ok(v) => v,
    Err(_e) => {
      tracing::warn!("未知的数据包类型，请更新本消息源，若已是最新请等待适配");
      return Ok(());
    }
  };
  match packet {
    either::Left(msg) => {
      info!("收到消息：");
      println!("{}", serde_json::to_string_pretty(&msg).unwrap());
    }
    either::Right(event) => {
      info!("收到事件");
      println!("{:?}", serde_json::to_string_pretty(&event).unwrap());
    }
  }
  Ok(())
}
