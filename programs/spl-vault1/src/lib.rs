use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
declare_id!("8ciPzBsxujt9z1YX544mtsnzZcyVpQ4f5aAiKRmE3FHv");

#[program]
pub mod spl_vault1 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, amount: u64) -> Result<()> {
        ctx.accounts.initialize(amount, &ctx.bumps)?;
        Ok(())
    }

    pub fn deposit(ctx: Context<Operations>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<Operations>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(constraint = mint.is_initialized == true)]
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = user
    )]
    pub owner_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        seeds=[b"state".as_ref(),user.key().as_ref(),mint.key().as_ref()],
        bump,
        space = VaultState::INIT_SPACE
    )]
    pub state: Account<'info, VaultState>,

    #[account(
        seeds=[b"vault".as_ref(),state.key().as_ref()],
        bump,
    )]
    /// CHECK: This is a PDA authority, validated by program-derived seeds
    pub vault_authority: UncheckedAccount<'info>,

    //vault's token account (ATA)-anchor creates if not present
    #[account(
        init ,
        payer = user,
        associated_token::mint=mint,
        associated_token::authority = vault_authority

    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    //programs
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, amount: u64, bumps: &InitializeBumps) -> Result<()> {
        self.state.amount = amount;
        self.state.vault_bump = bumps.vault_authority;
        self.state.state_bump = bumps.state;
        Ok(())
    }
}

#[derive(Accounts)]

pub struct Operations<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = user
    )]
    pub owner_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds=[b"state".as_ref(),user.key().as_ref(),mint.key().as_ref()],
        bump = state.state_bump,
    )]
    pub state: Account<'info, VaultState>,

    #[account(
        seeds=[b"vault".as_ref(),state.key().as_ref()],
        bump,
    )]

    /// CHECK: This is a PDA authority, validated by program-derived seeds
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint=mint,
        associated_token::authority = vault_authority

    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    // Token program for CPI transfers
    pub token_program: Program<'info, Token>,
}

impl<'info> Operations<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.owner_token_account.to_account_info(),
            to: self.vault_token_account.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, amount)?;
        Ok(())
    }
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let vault_bump = self.state.vault_bump;
        let state_key = self.state.key(); // state.key allows multiple vaults per user
        let seeds = &[b"vault".as_ref(), state_key.as_ref(), &[vault_bump]];
        let signer_seeds = &[&seeds[..]];

        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            to: self.owner_token_account.to_account_info(),
            from: self.vault_token_account.to_account_info(),
            authority: self.vault_authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[account]

pub struct VaultState {
    pub amount: u64,
    pub vault_bump: u8,
    pub state_bump: u8,
}

impl Space for VaultState {
    const INIT_SPACE: usize = 8 + 8 + 1 + 1;
}
