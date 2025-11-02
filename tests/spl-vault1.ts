import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SplVault1 } from "../target/types/spl_vault1";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("spl_vault", () => {
  // set up local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.spl_vault1 as Program<SplVault1>;
  const user = provider.wallet.publicKey;
  let mint: PublicKey;
  let userAta;
  let statePda;
  let vaultAta;
  let vaultAuthorityPda;



  it("Initialize vault", async () => {

    // create mint (9 decimals)
    mint = await createMint(
      provider.connection,
      provider.wallet.payer,
      user,
      null,
      9
    );
    console.log("mint: {}", mint);

    // token account for user
    userAta = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      mint,
      user
    );
    console.log("userAta: {}", userAta);

    // mint token to user
    await mintTo(
      provider.connection,
      provider.wallet.payer,
      mint,
      userAta.address,
      user,
      BigInt(1_000_000_000) // 1 token (9 decimals)
    );

    // Derive PDAs
    [statePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("state"), user.toBuffer(), mint.toBuffer()],
      program.programId
    );

    [vaultAuthorityPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), statePda.toBuffer()],
      program.programId
    );

    const vaultAtaAddress = await anchor.utils.token.associatedAddress({
      mint,
      owner: vaultAuthorityPda,
    })

    //call initialize
    await program.methods
      .initialize(new anchor.BN(0))
      .accountsStrict({
        user,
        mint,
        ownerTokenAccount: userAta.address,
        state: statePda,
        vaultAuthority: vaultAuthorityPda,
        vaultTokenAccount: vaultAtaAddress,
        systemProgram: anchor.web3.SystemProgram.programId,
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    // Verify ATA was created
    const vaultInfo = await getAccount(provider.connection, vaultAtaAddress)
    vaultAta = vaultInfo.address;
    console.log("vaultAta :", vaultAta);
    assert.ok(vaultInfo.amount == BigInt(0));

  });

  it("Deposit tokens", async () => {
    await program.methods
      .deposit(new anchor.BN(1000_000_000)) // deposit
      .accountsStrict({
        user, mint,
        ownerTokenAccount: userAta.address,
        state: statePda,
        vaultAuthority: vaultAuthorityPda,
        vaultTokenAccount: vaultAta,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    const userAccount = await getAccount(provider.connection, userAta.address);
    const vaultAccount = await getAccount(provider.connection, vaultAta);
    assert.strictEqual(Number(userAccount.amount), 0);
    assert.strictEqual(Number(vaultAccount.amount), 1000_000_000);
    console.log("user wallet after deposit:", userAccount.amount);
    console.log("Vault after deposit:", vaultAccount.amount);

  })

  it("Withdraw tokens", async () => {
    await program.methods
      .withdraw(new anchor.BN(200_000_000))
      .accountsStrict({
        user,
        mint,
        ownerTokenAccount: userAta.address,
        state: statePda,
        vaultAuthority: vaultAuthorityPda,
        vaultTokenAccount: vaultAta,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .rpc();

    const userAccount = await getAccount(provider.connection, userAta.address);
    const vaultAccount = await getAccount(provider.connection, vaultAta);

    // After withdrawing 0.2 token (200_000_000)
    assert.strictEqual(Number(userAccount.amount), 200_000_000);
    assert.strictEqual(Number(vaultAccount.amount), 800_000_000);
    console.log("user wallet after withdraw:", userAccount.amount);
    console.log("Vault after withdraw:", vaultAccount.amount);

  })
});
