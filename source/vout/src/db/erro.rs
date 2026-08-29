use sea_orm::SqlErr;

#[inline]
pub fn handle<T>(err: &anyhow::Result<T>) -> Option<SqlErr> {
    if let Err(err) = err {
        if let Some(err) = err.downcast_ref::<sea_orm::DbErr>() {
            return err.sql_err();
        }
    }

    None
}
