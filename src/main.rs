use std::str::FromStr;
use axum::Router;


use chrono::TimeDelta;
use shuttle_axum::ShuttleAxum;
use shuttle_secrets::SecretStore;
use teloxide::{prelude::*, types::ChatPermissions, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    parse_with = "split",
    description = "Команды для администратора."
)]
enum Command {
    #[command(description = "Показать справку.")]
    Help,
    #[command(
        description = "Размутить пользователя. Кидается в ответ на сообщение пользователя, которого надо размутить."
    )]
    Unmute,
    #[command(description = "Замутить пользователя. \
    Кидается в ответ на сообщение пользователя, которого надо замутить. \
    После команды нужно указать число и единицу измерения времени - на сколько замутить человека. ")]
    Mute { time: u64, unit: UnitOfTime },
}

#[derive(Clone)]
enum UnitOfTime {
    Seconds,
    Minutes,
    Hours,
}

impl FromStr for UnitOfTime {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        match s {
            "h" | "hours" | "ч" => Ok(UnitOfTime::Hours),
            "m" | "minutes" | "м" => Ok(UnitOfTime::Minutes),
            "s" | "seconds" | "с" => Ok(UnitOfTime::Seconds),
            _ => Err("Разрешённые единицы: h, m, s"),
        }
    }
}

pub async fn build_router(api_key_for_some_service: String) -> Router {
    let bot = Bot::new(api_key_for_some_service);
    Command::repl(bot, action).await;
    Router::new()
}

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_secrets::Secrets] secret_store: SecretStore
) -> ShuttleAxum {
    let my_secret = secret_store.get("TELOXIDE_TOKEN").unwrap();

    // Use the shared build function
    let router = build_router(my_secret).await;

    Ok(router.into())
}

async fn action(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Unmute => unmute_user(bot, msg).await?,
        Command::Mute { time, unit } => mute_user(bot, msg, calc_restrict_time(time, unit)).await?,
    };

    Ok(())
}

async fn mute_user(bot: Bot, msg: Message, time: TimeDelta) -> ResponseResult<()> {
    let callee_id = msg.from().unwrap().id;
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let mut is_current_user_admin = false;
    for admin in admins.iter() {
        if admin.user.id == callee_id {
            is_current_user_admin = true;
        }
    }
    if !is_current_user_admin {
        bot.send_message(
            msg.chat.id,
            "У вас недостаточно прав для совершения операции.",
        )
        .reply_to_message_id(msg.id)
        .await?;
    }
    match msg.reply_to_message() {
        Some(replied) => 'lol: {
            let mentioned_person = replied.from().unwrap().id;
            let mut is_mentioned_admin = false;
            for admin in admins.iter() {
                if admin.user.id == mentioned_person {
                    is_mentioned_admin = true;
                }
            }
            if is_mentioned_admin {
                bot.send_message(msg.chat.id, "Пользователь является администратором. Увы.")
                    .reply_to_message_id(msg.id)
                    .await?;
                break 'lol;
            }
            bot.restrict_chat_member(
                msg.chat.id,
                replied.from().expect("Must be MessageKind::Common").id,
                ChatPermissions::empty(),
            )
            .until_date(msg.date + time)
            .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Используй эту команду в ответ на сообщение пользователя, которого хочешь замутить!")
                .await?;
        }
    }
    Ok(())
}

async fn unmute_user(bot: Bot, msg: Message) -> ResponseResult<()> {
    let callee_id = msg.from().unwrap().id;
    let admins = bot.get_chat_administrators(msg.chat.id).await?;
    let mut is_current_user_admin = false;
    for admin in admins.iter() {
        if admin.user.id == callee_id {
            is_current_user_admin = true;
        }
    }
    if !is_current_user_admin {
        bot.send_message(
            msg.chat.id,
            "У вас недостаточно прав для совершения операции.",
        )
        .reply_to_message_id(msg.id)
        .await?;
    }
    match msg.reply_to_message() {
        Some(replied) => 'lol: {
            let mentioned_person = replied.from().unwrap().id;
            let mut is_mentioned_admin = false;
            for admin in admins.iter() {
                if admin.user.id == mentioned_person {
                    is_mentioned_admin = true;
                }
            }
            if is_mentioned_admin {
                bot.send_message(msg.chat.id, "Пользователь является администратором. Увы.")
                    .reply_to_message_id(msg.id)
                    .await?;
                break 'lol;
            }
            bot.restrict_chat_member(
                msg.chat.id,
                replied.from().expect("Must be MessageKind::Common").id,
                ChatPermissions::all(),
            )
            .await?;
        }
        None => {
            bot
                .send_message(
                    msg.chat.id,
                    "Используй эту команду в ответ на сообщение пользователя, которого хочешь размутить!",
                )
                .await?;
        }
    }
    Ok(())
}

fn calc_restrict_time(time: u64, unit: UnitOfTime) -> TimeDelta {
    match unit {
        UnitOfTime::Hours => TimeDelta::try_hours(time as i64).unwrap(),
        UnitOfTime::Minutes => TimeDelta::try_minutes(time as i64).unwrap(),
        UnitOfTime::Seconds => TimeDelta::try_seconds(time as i64).unwrap(),
    }
}
