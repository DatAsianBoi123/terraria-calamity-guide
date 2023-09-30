use rocket::{response::Redirect, get};

#[get("/invite")]
pub async fn invite() -> Redirect {
    Redirect::to("https://discord.com/api/oauth2/authorize?client_id=1128716845365596273&permissions=274878171136&scope=bot%20applications.commands")
}

