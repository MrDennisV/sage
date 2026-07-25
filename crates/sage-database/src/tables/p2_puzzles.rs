use chia_bls::PublicKey;
use chia_protocol::Bytes32;
use chia_sdk_driver::{ClawbackV2, OptionType, OptionUnderlying};
use chia_sdk_types::{Mod, puzzles::P2DelegatedConditionsArgs};
use clvm_utils::ToTreeHash;

use crate::{
    Convert, Database, DatabaseError, DatabaseTx, Result, SqlAccess, SqlExecutor, sql_file,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum P2PuzzleKind {
    PublicKey,
    Clawback,
    Option,
    Arbor,
}

#[derive(Debug, Clone, Copy)]
pub enum P2Puzzle {
    PublicKey(PublicKey),
    Clawback(Clawback),
    Option(Underlying),
    Arbor(PublicKey),
}

#[derive(Debug, Clone, Copy)]
pub struct Clawback {
    pub public_key: Option<PublicKey>,
    pub sender_puzzle_hash: Bytes32,
    pub receiver_puzzle_hash: Bytes32,
    pub seconds: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct Underlying {
    pub public_key: PublicKey,
    pub launcher_id: Bytes32,
    pub creator_puzzle_hash: Bytes32,
    pub seconds: u64,
    pub amount: u64,
    pub strike_type: OptionType,
}

#[derive(Debug, Clone, Copy)]
pub struct Derivation {
    pub derivation_index: u32,
    pub is_hardened: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DerivationRow {
    pub p2_puzzle_hash: Bytes32,
    pub index: u32,
    pub hardened: bool,
    pub synthetic_key: PublicKey,
}

impl<E: SqlExecutor> Database<E> {
    pub async fn derivations(
        &self,
        is_hardened: bool,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<DerivationRow>, u32)> {
        derivations(&self.executor, is_hardened, limit, offset).await
    }

    pub async fn max_derivation_index(&self, is_hardened: bool) -> Result<Option<u32>> {
        max_derivation_index(&self.executor, is_hardened).await
    }

    pub async fn custody_p2_puzzle_hashes(&self) -> Result<Vec<Bytes32>> {
        custody_p2_puzzle_hashes(&self.executor).await
    }

    pub async fn is_p2_puzzle_hash(&self, puzzle_hash: Bytes32) -> Result<bool> {
        is_p2_puzzle_hash(&self.executor, puzzle_hash).await
    }

    pub async fn public_key(&self, p2_puzzle_hash: Bytes32) -> Result<Option<PublicKey>> {
        public_key(&self.executor, p2_puzzle_hash).await
    }

    pub async fn is_custody_p2_puzzle_hash(&self, puzzle_hash: Bytes32) -> Result<bool> {
        is_custody_p2_puzzle_hash(&self.executor, puzzle_hash).await
    }

    pub async fn p2_puzzle(&self, puzzle_hash: Bytes32) -> Result<P2Puzzle> {
        match p2_puzzle_kind(&self.executor, puzzle_hash).await? {
            P2PuzzleKind::PublicKey => {
                let Some(key) = public_key(&self.executor, puzzle_hash).await? else {
                    return Err(DatabaseError::PublicKeyNotFound);
                };

                Ok(P2Puzzle::PublicKey(key))
            }
            P2PuzzleKind::Clawback => Ok(P2Puzzle::Clawback(
                clawback(&self.executor, puzzle_hash).await?,
            )),
            P2PuzzleKind::Option => {
                let launcher_id = underlying_launcher_id(&self.executor, puzzle_hash).await?;
                let underlying = self
                    .option_underlying(launcher_id)
                    .await?
                    .ok_or(DatabaseError::OptionUnderlyingNotFound)?;

                let Some(key) = public_key(&self.executor, underlying.creator_puzzle_hash).await?
                else {
                    return Err(DatabaseError::PublicKeyNotFound);
                };

                Ok(P2Puzzle::Option(Underlying {
                    public_key: key,
                    launcher_id,
                    creator_puzzle_hash: underlying.creator_puzzle_hash,
                    seconds: underlying.seconds,
                    amount: underlying.amount,
                    strike_type: underlying.strike_type,
                }))
            }
            P2PuzzleKind::Arbor => {
                let Some(key) = arbor_key(&self.executor, puzzle_hash).await? else {
                    return Err(DatabaseError::PublicKeyNotFound);
                };

                Ok(P2Puzzle::Arbor(key))
            }
        }
    }

    pub async fn derivation(&self, public_key: PublicKey) -> Result<Option<Derivation>> {
        derivation(&self.executor, public_key).await
    }
}

impl<E: SqlExecutor> DatabaseTx<'_, E> {
    pub async fn insert_clawback_p2_puzzle(&mut self, clawback: ClawbackV2) -> Result<()> {
        insert_clawback_p2_puzzle(&mut self.tx, clawback).await
    }

    pub async fn insert_option_p2_puzzle(&mut self, underlying: OptionUnderlying) -> Result<()> {
        insert_option_p2_puzzle(&mut self.tx, underlying).await
    }

    pub async fn insert_arbor_p2_puzzle(&mut self, key: PublicKey) -> Result<()> {
        insert_arbor_p2_puzzle(&mut self.tx, key).await
    }

    pub async fn is_p2_puzzle_hash(&mut self, puzzle_hash: Bytes32) -> Result<bool> {
        is_p2_puzzle_hash(&mut self.tx, puzzle_hash).await
    }

    pub async fn insert_custody_p2_puzzle(
        &mut self,
        p2_puzzle_hash: Bytes32,
        key: PublicKey,
        derivation: Derivation,
    ) -> Result<()> {
        insert_custody_p2_puzzle(&mut self.tx, p2_puzzle_hash, key, derivation).await
    }

    pub async fn custody_p2_puzzle_hash(
        &mut self,
        derivation_index: u32,
        is_hardened: bool,
    ) -> Result<Bytes32> {
        custody_p2_puzzle_hash(&mut self.tx, derivation_index, is_hardened).await
    }

    pub async fn is_custody_p2_puzzle_hash(&mut self, puzzle_hash: Bytes32) -> Result<bool> {
        is_custody_p2_puzzle_hash(&mut self.tx, puzzle_hash).await
    }

    pub async fn unused_derivation_index(&mut self, is_hardened: bool) -> Result<u32> {
        unused_derivation_index(&mut self.tx, is_hardened).await
    }

    pub async fn derivation_index(&mut self, is_hardened: bool) -> Result<u32> {
        derivation_index(&mut self.tx, is_hardened).await
    }
}

async fn custody_p2_puzzle_hashes(mut conn: impl SqlAccess) -> Result<Vec<Bytes32>> {
    conn.fetch_all(sql_file!("p2_puzzles/custody_p2_puzzle_hashes.sql"), vec![])
        .await?
        .iter()
        .map(|row| row.converted("hash"))
        .collect()
}

async fn custody_p2_puzzle_hash(
    mut conn: impl SqlAccess,
    derivation_index: u32,
    is_hardened: bool,
) -> Result<Bytes32> {
    conn.fetch_all(
        sql_file!("p2_puzzles/custody_p2_puzzle_hash.sql"),
        vec![derivation_index.into(), is_hardened.into()],
    )
    .await?
    .first()
    .ok_or(DatabaseError::RowNotFound)?
    .converted("hash")
}

async fn is_custody_p2_puzzle_hash(mut conn: impl SqlAccess, puzzle_hash: Bytes32) -> Result<bool> {
    Ok(conn
        .fetch_all(
            sql_file!("p2_puzzles/is_custody_p2_puzzle_hash.sql"),
            vec![puzzle_hash.into()],
        )
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        > 0)
}

async fn is_p2_puzzle_hash(mut conn: impl SqlAccess, puzzle_hash: Bytes32) -> Result<bool> {
    Ok(conn
        .fetch_all(
            sql_file!("p2_puzzles/is_p2_puzzle_hash.sql"),
            vec![puzzle_hash.into()],
        )
        .await?
        .first()
        .ok_or(DatabaseError::RowNotFound)?
        .i64("count")?
        > 0)
}

async fn derivation_index(mut conn: impl SqlAccess, is_hardened: bool) -> Result<u32> {
    conn.fetch_all(
        sql_file!("p2_puzzles/derivation_index.sql"),
        vec![is_hardened.into()],
    )
    .await?
    .first()
    .ok_or(DatabaseError::RowNotFound)?
    .i64("derivation_index")?
    .convert()
}

async fn max_derivation_index(mut conn: impl SqlAccess, is_hardened: bool) -> Result<Option<u32>> {
    conn.fetch_all(
        sql_file!("p2_puzzles/max_derivation_index.sql"),
        vec![is_hardened.into()],
    )
    .await?
    .first()
    .ok_or(DatabaseError::RowNotFound)?
    .opt_i64("derivation_index")?
    .convert()
}

async fn derivations(
    mut conn: impl SqlAccess,
    is_hardened: bool,
    limit: u32,
    offset: u32,
) -> Result<(Vec<DerivationRow>, u32)> {
    let rows = conn
        .fetch_all(
            sql_file!("p2_puzzles/derivations.sql"),
            vec![is_hardened.into(), limit.into(), offset.into()],
        )
        .await?;

    let total_count = rows
        .first()
        .map_or(Ok(0), |row| row.i64("total")?.convert())?;

    let derivations = rows
        .iter()
        .map(|row| {
            Ok(DerivationRow {
                p2_puzzle_hash: row.converted("p2_puzzle_hash")?,
                index: row.i64("derivation_index")?.convert()?,
                hardened: row.i64("is_hardened")? != 0,
                synthetic_key: row.converted("synthetic_key")?,
            })
        })
        .collect::<Result<Vec<DerivationRow>>>()?;

    Ok((derivations, total_count))
}

async fn unused_derivation_index(mut conn: impl SqlAccess, is_hardened: bool) -> Result<u32> {
    conn.fetch_all(
        sql_file!("p2_puzzles/unused_derivation_index.sql"),
        vec![is_hardened.into()],
    )
    .await?
    .first()
    .ok_or(DatabaseError::RowNotFound)?
    .i64("derivation_index")?
    .convert()
}

async fn insert_custody_p2_puzzle(
    mut conn: impl SqlAccess,
    p2_puzzle_hash: Bytes32,
    key: PublicKey,
    derivation: Derivation,
) -> Result<()> {
    conn.execute(
        sql_file!("p2_puzzles/insert_custody_p2_puzzle.sql"),
        vec![
            p2_puzzle_hash.into(),
            p2_puzzle_hash.into(),
            derivation.is_hardened.into(),
            derivation.derivation_index.into(),
            key.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_clawback_p2_puzzle(mut conn: impl SqlAccess, clawback: ClawbackV2) -> Result<()> {
    let p2_puzzle_hash = clawback.tree_hash().to_vec();
    let seconds: i64 = clawback.seconds.try_into()?;

    conn.execute(
        sql_file!("p2_puzzles/insert_clawback_p2_puzzle.sql"),
        vec![
            p2_puzzle_hash.clone().into(),
            p2_puzzle_hash.into(),
            clawback.sender_puzzle_hash.into(),
            clawback.receiver_puzzle_hash.into(),
            seconds.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_option_p2_puzzle(
    mut conn: impl SqlAccess,
    underlying: OptionUnderlying,
) -> Result<()> {
    let p2_puzzle_hash = underlying.tree_hash().to_vec();
    let seconds: i64 = underlying.seconds.try_into()?;

    conn.execute(
        sql_file!("p2_puzzles/insert_option_p2_puzzle.sql"),
        vec![
            p2_puzzle_hash.clone().into(),
            p2_puzzle_hash.into(),
            underlying.launcher_id.into(),
            underlying.creator_puzzle_hash.into(),
            seconds.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn insert_arbor_p2_puzzle(mut conn: impl SqlAccess, key: PublicKey) -> Result<()> {
    let p2_puzzle_hash = P2DelegatedConditionsArgs::new(key)
        .curry_tree_hash()
        .to_vec();

    conn.execute(
        sql_file!("p2_puzzles/insert_arbor_p2_puzzle.sql"),
        vec![
            p2_puzzle_hash.clone().into(),
            p2_puzzle_hash.into(),
            key.into(),
        ],
    )
    .await?;

    Ok(())
}

async fn p2_puzzle_kind(mut conn: impl SqlAccess, p2_puzzle_hash: Bytes32) -> Result<P2PuzzleKind> {
    let rows = conn
        .fetch_all(
            sql_file!("p2_puzzles/p2_puzzle_kind.sql"),
            vec![p2_puzzle_hash.into()],
        )
        .await?;

    let row = rows.first().ok_or(DatabaseError::RowNotFound)?;

    Ok(match row.i64("kind")? {
        0 => P2PuzzleKind::PublicKey,
        1 => P2PuzzleKind::Clawback,
        2 => P2PuzzleKind::Option,
        3 => P2PuzzleKind::Arbor,
        _ => return Err(DatabaseError::InvalidEnumVariant),
    })
}

async fn public_key(
    mut conn: impl SqlAccess,
    p2_puzzle_hash: Bytes32,
) -> Result<Option<PublicKey>> {
    conn.fetch_all(
        sql_file!("p2_puzzles/public_key.sql"),
        vec![p2_puzzle_hash.into()],
    )
    .await?
    .first()
    .map(|row| row.converted("key"))
    .transpose()
}

async fn clawback(mut conn: impl SqlAccess, p2_puzzle_hash: Bytes32) -> Result<Clawback> {
    let rows = conn
        .fetch_all(
            sql_file!("p2_puzzles/clawback.sql"),
            vec![p2_puzzle_hash.into()],
        )
        .await?;

    let row = rows.first().ok_or(DatabaseError::RowNotFound)?;

    Ok(Clawback {
        public_key: row.opt_converted("key")?,
        sender_puzzle_hash: row.converted("sender_puzzle_hash")?,
        receiver_puzzle_hash: row.converted("receiver_puzzle_hash")?,
        seconds: row.i64("expiration_seconds")?.convert()?,
    })
}

async fn underlying_launcher_id(
    mut conn: impl SqlAccess,
    p2_puzzle_hash: Bytes32,
) -> Result<Bytes32> {
    conn.fetch_all(
        sql_file!("p2_puzzles/underlying_launcher_id.sql"),
        vec![p2_puzzle_hash.into()],
    )
    .await?
    .first()
    .ok_or(DatabaseError::RowNotFound)?
    .converted("launcher_id")
}

async fn arbor_key(mut conn: impl SqlAccess, p2_puzzle_hash: Bytes32) -> Result<Option<PublicKey>> {
    conn.fetch_all(
        sql_file!("p2_puzzles/arbor_key.sql"),
        vec![p2_puzzle_hash.into()],
    )
    .await?
    .first()
    .map(|row| row.converted("key"))
    .transpose()
}

async fn derivation(mut conn: impl SqlAccess, public_key: PublicKey) -> Result<Option<Derivation>> {
    conn.fetch_all(
        sql_file!("p2_puzzles/derivation.sql"),
        vec![public_key.into()],
    )
    .await?
    .first()
    .map(|row| {
        Ok(Derivation {
            derivation_index: row.i64("derivation_index")?.convert()?,
            is_hardened: row.i64("is_hardened")? != 0,
        })
    })
    .transpose()
}
