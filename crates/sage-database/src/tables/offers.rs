use chia_protocol::Bytes32;

use crate::{
    Asset, Convert, Database, DatabaseError, DatabaseTx, Result, SqlAccess, SqlExecutor, SqlRow,
    sql_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum OfferStatus {
    Pending = 0,
    Active = 1,
    Completed = 2,
    Cancelled = 3,
    Expired = 4,
}

#[derive(Debug, Clone)]
pub struct OfferRow {
    pub offer_id: Bytes32,
    pub encoded_offer: String,
    pub expiration_height: Option<u32>,
    pub expiration_timestamp: Option<u64>,
    pub fee: u64,
    pub status: OfferStatus,
    pub inserted_timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct OfferedAsset {
    pub offer_id: Bytes32,
    pub asset: Asset,
    pub is_requested: bool,
    pub amount: u64,
    pub royalty: u64,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn offer(&self, offer_id: Bytes32) -> Result<Option<OfferRow>> {
        offer(&self.executor, offer_id).await
    }

    pub async fn offer_assets(&self, offer_id: Bytes32) -> Result<Vec<OfferedAsset>> {
        offer_assets(&self.executor, offer_id).await
    }

    pub async fn delete_offer(&self, offer_id: Bytes32) -> Result<()> {
        delete_offer(&self.executor, offer_id).await
    }

    pub async fn offers(&self, status: Option<OfferStatus>) -> Result<Vec<OfferRow>> {
        offers(&self.executor, status).await
    }

    pub async fn update_offer_status(&self, offer_id: Bytes32, status: OfferStatus) -> Result<()> {
        update_offer_status(&self.executor, offer_id, status).await
    }

    pub async fn offers_for_asset(
        &self,
        asset_id: Bytes32,
        status: Option<OfferStatus>,
    ) -> Result<Vec<OfferRow>> {
        offers_for_asset(&self.executor, asset_id, status).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_offer(&mut self, offer: OfferRow) -> Result<()> {
        insert_offer(&mut self.tx, offer).await
    }

    pub async fn insert_offered_coin(&mut self, offer_id: Bytes32, coin_id: Bytes32) -> Result<()> {
        insert_offered_coin(&mut self.tx, offer_id, coin_id).await
    }

    pub async fn insert_offer_asset(
        &mut self,
        offer_id: Bytes32,
        asset_id: Bytes32,
        amount: u64,
        royalty: u64,
        is_requested: bool,
    ) -> Result<()> {
        insert_offer_asset(
            &mut self.tx,
            offer_id,
            asset_id,
            amount,
            royalty,
            is_requested,
        )
        .await
    }

    pub async fn update_offer_status(
        &mut self,
        offer_id: Bytes32,
        status: OfferStatus,
    ) -> Result<()> {
        update_offer_status(&mut self.tx, offer_id, status).await
    }

    pub async fn offers_for_asset(
        &mut self,
        asset_id: Bytes32,
        status: Option<OfferStatus>,
    ) -> Result<Vec<OfferRow>> {
        offers_for_asset(&mut self.tx, asset_id, status).await
    }
}

/// Decodes the columns shared by the `offer`, `offers` and `offers_for_asset`
/// queries.
fn offer_row_from_row(row: &SqlRow) -> Result<OfferRow> {
    Ok(OfferRow {
        offer_id: row.converted("offer_id")?,
        encoded_offer: row.text("encoded_offer")?,
        expiration_height: row.opt_i64("expiration_height")?.map(|h| h as u32),
        expiration_timestamp: row.opt_i64("expiration_timestamp")?.map(|t| t as u64),
        fee: row.converted("fee")?,
        status: match row.i64("status")? {
            0 => OfferStatus::Pending,
            1 => OfferStatus::Active,
            2 => OfferStatus::Completed,
            3 => OfferStatus::Cancelled,
            4 => OfferStatus::Expired,
            _ => return Err(DatabaseError::InvalidEnumVariant),
        },
        inserted_timestamp: row.i64("inserted_timestamp")? as u64,
    })
}

async fn offers_for_asset(
    mut conn: impl SqlAccess,
    asset_id: Bytes32,
    status: Option<OfferStatus>,
) -> Result<Vec<OfferRow>> {
    let status_value = status.map(|s| s as u8);

    conn.fetch_all(
        sql_file!("offers/offers_for_asset.sql"),
        vec![asset_id.into(), status_value.into(), status_value.into()],
    )
    .await?
    .iter()
    .map(offer_row_from_row)
    .collect()
}

async fn offer_assets(mut conn: impl SqlAccess, offer_id: Bytes32) -> Result<Vec<OfferedAsset>> {
    conn.fetch_all(sql_file!("offers/offer_assets.sql"), vec![offer_id.into()])
        .await?
        .iter()
        .map(|row| {
            Ok(OfferedAsset {
                offer_id: row.converted("offer_id")?,
                asset: Asset {
                    hash: row.converted("asset_id")?,
                    description: row.opt_text("description")?,
                    is_sensitive_content: row.i64("is_sensitive_content")? != 0,
                    is_visible: row.i64("is_visible")? != 0,
                    icon_url: row.opt_text("icon_url")?,
                    kind: row.i64("kind")?.convert()?,
                    name: row.opt_text("name")?,
                    ticker: row.opt_text("ticker")?,
                    precision: row.i64("precision")?.convert()?,
                    hidden_puzzle_hash: row.opt_converted("hidden_puzzle_hash")?,
                },
                amount: row.converted("amount")?,
                royalty: row.converted("royalty")?,
                is_requested: row.i64("is_requested")? != 0,
            })
        })
        .collect()
}

async fn insert_offer(mut conn: impl SqlAccess, offer: OfferRow) -> Result<()> {
    let expiration_height: Option<i64> = offer.expiration_height.map(Into::into);
    let expiration_timestamp: Option<i64> = offer
        .expiration_timestamp
        .map(TryInto::try_into)
        .transpose()?;
    let inserted_timestamp: i64 = offer.inserted_timestamp.try_into()?;
    let fee = offer.fee.to_be_bytes().to_vec();

    conn.execute(
        sql_file!("offers/insert_offer.sql"),
        vec![
            offer.offer_id.into(),
            offer.encoded_offer.into(),
            fee.into(),
            (offer.status as u8).into(),
            expiration_height.into(),
            expiration_timestamp.into(),
            inserted_timestamp.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_offer_asset(
    mut conn: impl SqlAccess,
    offer_id: Bytes32,
    asset_id: Bytes32,
    amount: u64,
    royalty: u64,
    is_requested: bool,
) -> Result<()> {
    let amount = amount.to_be_bytes().to_vec();
    let royalty = royalty.to_be_bytes().to_vec();

    conn.execute(
        sql_file!("offers/insert_offer_asset.sql"),
        vec![
            offer_id.into(),
            asset_id.into(),
            amount.into(),
            royalty.into(),
            is_requested.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_offered_coin(
    mut conn: impl SqlAccess,
    offer_hash: Bytes32,
    coin_hash: Bytes32,
) -> Result<()> {
    conn.execute(
        sql_file!("offers/insert_offered_coin.sql"),
        vec![offer_hash.into(), coin_hash.into()],
    )
    .await?;

    Ok(())
}

async fn offer(mut conn: impl SqlAccess, offer_id: Bytes32) -> Result<Option<OfferRow>> {
    conn.fetch_all(sql_file!("offers/offer.sql"), vec![offer_id.into()])
        .await?
        .first()
        .map(offer_row_from_row)
        .transpose()
}

async fn offers(mut conn: impl SqlAccess, status: Option<OfferStatus>) -> Result<Vec<OfferRow>> {
    let status_value = status.map(|s| s as u8);

    conn.fetch_all(
        sql_file!("offers/offers.sql"),
        vec![status_value.into(), status_value.into()],
    )
    .await?
    .iter()
    .map(offer_row_from_row)
    .collect()
}

async fn delete_offer(mut conn: impl SqlAccess, offer_id: Bytes32) -> Result<()> {
    conn.execute(sql_file!("offers/delete_offer.sql"), vec![offer_id.into()])
        .await?;

    Ok(())
}

async fn update_offer_status(
    mut conn: impl SqlAccess,
    offer_id: Bytes32,
    status: OfferStatus,
) -> Result<()> {
    conn.execute(
        sql_file!("offers/update_offer_status.sql"),
        vec![(status as u8).into(), offer_id.to_vec().into()],
    )
    .await?;

    Ok(())
}
