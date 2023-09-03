use std::time::Duration;

use actix_web::{get, middleware, web, App, HttpServer};
use anyhow::anyhow;
use log::{error, info};
use sea_orm::{Database, DatabaseConnection};
use sea_orm::entity::*;
use sea_orm::query::*;
use sea_orm::sea_query::{*, ColumnDef};
use tokio::sync::watch;
use tokio::time;

static DATABASE_URL: &str = "mysql://USERNAME:PASSWORD@127.0.0.1/DATABASE";

mod something {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "something")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

struct Context {
    stop_tx: watch::Sender<bool>,
    stop_rx: watch::Receiver<bool>,
    db: DatabaseConnection,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or("info")
    );

    let (stop_tx, stop_rx) = watch::channel(false);
    let db = Database::connect(DATABASE_URL).await?;
    let ctx = web::Data::new(Context {
        stop_tx,
        stop_rx,
        db,
    });

    let stmt = ctx.db.get_database_backend().build(
        Table::create()
            .table(Alias::new("something"))
            .if_not_exists()
            .col(ColumnDef::new(Alias::new("id")).integer().not_null().auto_increment().primary_key())
    );
    ctx.db.execute(stmt).await?;

    let queue_handler = tokio::spawn(background_task(ctx.clone()));

    start_server(ctx.clone()).await?;

    ctx.stop_tx.send(true)?;

    queue_handler.await?;

    Ok(())
}

async fn start_server(ctx: web::Data<Context>) -> anyhow::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(ctx.clone())
            .wrap(middleware::Logger::default())
            .service(test_request)
    })
    .bind(("127.0.0.1", 8080))
    .map_err(|e| anyhow!("cannot bind server: {}", e))?
    .run().await?;
    Ok(())
}

#[get("/test")]
async fn test_request(ctx: web::Data<Context>) -> &'static str {
    //executes: INSERT INTO something VALUES ()
    something::ActiveModel {
        ..Default::default()
    }.insert(&ctx.db).await.unwrap();
    ""
}

async fn background_task(ctx: web::Data<Context>) {
    let stop_rx = ctx.stop_rx.clone();
    while !*stop_rx.borrow() {
        if let Err(err) = do_stuff(&ctx).await {
            error!("error doing stuff: {}", err);
        }
    }
}

async fn do_stuff(ctx: &Context) -> anyhow::Result<()> {
    info!("starting transaction");
    let txn = ctx.db.begin().await?;

    info!("sleeping");
    time::sleep(Duration::from_secs(2)).await;

    info!("inserting");
    //executes: INSERT INTO something VALUES ()
    something::ActiveModel {
        ..Default::default()
    }.insert(&txn).await?;

    info!("commit");
    txn.commit().await?;

    info!("done");
    Ok(())
}
