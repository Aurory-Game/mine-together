use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "local-testing"))]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "AURYydfxJib1ZkTir1Jn1J9ECYUtjb6rKQVmtYaixWPP";
    pub const MINE_TOGETHER_PDA_SEED: &[u8] = b"MINEtogether";
}

#[cfg(feature = "local-testing")]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9";
    pub const MINE_TOGETHER_PDA_SEED: &[u8] = b"MINEtogether";
}

#[program]
pub mod mine_together {
    use super::*;
    pub fn initialize(
        ctx: Context<Initialize>,
        _nonce_mine_together: u8,
        _nonce_aury_vault: u8,
    ) -> ProgramResult {
        ctx.accounts.mine_together_account.admin_key = *ctx.accounts.initializer.key;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_nonce_mine_together: u8, _nonce_aury_vault: u8)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = initializer,
        seeds = [ constants::MINE_TOGETHER_PDA_SEED.as_ref() ],
        bump = _nonce_mine_together,
    )]
    pub mine_together_account: Box<Account<'info, MineTogetherAccount>>,

    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        token::mint = aury_mint,
        token::authority = aury_vault,
        seeds = [ constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap().as_ref() ],
        bump = _nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
#[derive(Default)]
pub struct MineTogetherAccount {
    pub admin_key: Pubkey,
    pub freeze_program: bool,
}

impl MineTogetherAccount {
    pub fn assert_admin(&self, signer: &Signer) -> ProgramResult {
        if self.admin_key != *signer.key {
            return Err(ErrorCode::NotAdmin.into());
        }

        Ok(())
    }

    pub fn assert_freeze_program(&self) -> ProgramResult {
        if self.freeze_program {
            return Err(ErrorCode::ProgramFreezed.into());
        }

        Ok(())
    }
}

#[error]
pub enum ErrorCode {
    #[msg("Not admin")]
    NotAdmin, // 6000, 0x1770
    #[msg("Program freezed")]
    ProgramFreezed, // 6001, 0x1771
    #[msg("Token transfer failed")]
    TokenTransferFailed, // 6002, 0x1772
}
