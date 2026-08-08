# S016315 parity review — slot 1

Reviewed only the pinned `include/uapi/linux/nfsacl.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the S016315 candidate and
candidate diff, the S016315 semantic-closure proposal, the frozen task rows,
and direct local NFS contexts.  No compiler, formatter, test, diagnostic, or
historical source was used.

## Finding P1 — upstream copyright notice omitted

- **Linux symbol:** `_UAPI__LINUX_NFSACL_H` (the sole header guard enclosing
  the source file).
- **SC1:** `SC1-702c937d45b0c6f73d80e73f937614af69706995c16b86959d267c76d4228da4`
  (S016315 scope semantic-completeness record).
- **Evidence:** The pinned header prologue, before the
  `_UAPI__LINUX_NFSACL_H` guard at lines 7–8, retains `(C) 2003 Andreas
  Gruenbacher <agruen@suse.de>`.  Candidate `nfsacl.rs` retains the SPDX line
  but omits that relevant upstream copyright notice.  The same notice remains
  in the direct wrapper context `include/linux/nfsacl.h:2-6`.
- **Required resolution:** Restore the upstream copyright notice in the Rust
  file.  This is required provenance retention, not an allowlisted branding
  change.

## Exhaustive macro and context audit

The candidate exports each of the 16 UAPI numeric macros under the unchanged
spelling with the exact pinned value:

| Linux macro(s) | pinned value(s) | candidate result |
| --- | --- | --- |
| `NFS_ACL_PROGRAM` | `100227` | exact |
| `ACLPROC2_NULL`, `ACLPROC2_GETACL`, `ACLPROC2_SETACL`, `ACLPROC2_GETATTR`, `ACLPROC2_ACCESS` | `0`, `1`, `2`, `3`, `4` | exact |
| `ACLPROC3_NULL`, `ACLPROC3_GETACL`, `ACLPROC3_SETACL` | `0`, `1`, `2` | exact |
| `NFS_ACL`, `NFS_ACLCNT`, `NFS_DFACL`, `NFS_DFACLCNT`, `NFS_ACL_MASK` | `0x0001`, `0x0002`, `0x0004`, `0x0008`, `0x000f` | exact |
| `NFS_ACL_DEFAULT` | `0x1000` | exact |

All pinned literals are unsuffixed integer constants within C `int` range;
the candidate's explicit `i32` constants preserve the required 32-bit signed
integer representation on both approved architectures.  No arithmetic,
overflow, allocation, locking, cleanup, or branch path exists in this header.
The source has no configuration conditional around any of the 16 macros, and
the candidate likewise exposes each unconditionally.

`_UAPI__LINUX_NFSACL_H` is a preprocessor idempotence guard, not a runtime or
linkage symbol.  Candidate Rust module loading provides the corresponding
single-definition property; it does not introduce a replacement exported
symbol.  The candidate resides at the required UAPI namespace path and adds
no branding.  The direct wrapper `include/linux/nfsacl.h:13` includes the
pinned UAPI header, while direct NFS consumers use `NFS_ACL_PROGRAM` as an RPC
program number in `fs/nfs/nfs3client.c:17-23` and `fs/nfsd/nfssvc.c:118-127`.
The candidate's `NFS_ACL_PROGRAM: i32 = 100227` retains that value and width.

## Semantic-closure mapping

The proposal's source-review values are supported by the pinned header and
candidate for both architectures; the separately reported P1 provenance defect
does not alter their semantic values.  Literal SC1 coverage is:

- Scope: `SC1-702c937d45b0c6f73d80e73f937614af69706995c16b86959d267c76d4228da4`.
- AArch64 guard/conditionals: `SC1-dad9a14e43b3ce24c6a872b6a4798953ef833d76e1347dd52021b93dfb7898a7`, `SC1-6a873f9bb607a2a339a2b122047769df8c459160cbe11b81157fb27f95f6e8fb`, `SC1-5afb648bedd2d66db661ab1f341d9512ab7c78ea751aff3f9dba63a4bb03139f`, `SC1-c5d39de4dee4c834ff56e03185f2e55433de10bf328432a37e104c3c2ec8099c`.
- AArch64 macros in source order (`NFS_ACL_PROGRAM` through `NFS_ACL_DEFAULT`), selection then status: `SC1-a0a37f97985c5a85947547ec0f4bf1f50e977faae14e14814daa5dad7bcc274d` / `SC1-1215d10e3797d6f3f40035df6c5682184068efa709555e94bccd15f9abc1166b`; `SC1-8348becfa7b891b76830f1556901de5db9171c06b47097359f75e5e9755d27b7` / `SC1-8eaa468c4361b914f1c92ec694dc8759f214194a0ebdd4540991664cb8082eaa`; `SC1-af9e566b574d3e72e88059d10ca1f00cdd5c25abb8ac6a0424957633b3d78a12` / `SC1-4c85ea95f9e1ccd9e9b9eeca224359135a07a3c31e6efad73f5b5b67f25f6644`; `SC1-18349a35b273e8be1fc90958d7cd8bf54cc255e4517198ba0dc37bb4bc2483e0` / `SC1-f48ef19303f81f3b6dd875f83d43b854c5e68f8626ff0ccc1d8b5c4c3627f766`; `SC1-38e16c5b3a534e1e3bd6399f2661f970c74ee74973f012f4790d8d082ca59657` / `SC1-0ca346694ba546dde8ba6209e4afc1ad2986037a47b365a73f08b7e54087fe48`; `SC1-c3a3c951324c3cb35648c0321da3da2f007d22842e8541ed4652d4341641be39` / `SC1-5b3d93d1087cf59a3c5529adb1136f7e5bf8641e8645acd46cfbdfa01765932e`; `SC1-fb97ab4798edf4ee948101d4dee03bc6a2311e7d610f7df8ec7715e40bee848e` / `SC1-63cdaf8ab73ee9f828883e9e18ed791b290c7254b618b19821cb6cf713e06a9f`; `SC1-249b6605e566c7e4eedaed49c03efddc30a14ae3d94c90977c472eb4ee656724` / `SC1-714d2ccc632d05c7fd479091d6d104d6eb2b117695b2c615a7d659f6a76d43a9`; `SC1-ec5d5763bad4a0f70244eafdc0abde1d06c1c4c873b10df939a94e5a3334f46c` / `SC1-5a9c75252b1d4498ce8905728e77e7518e7a28d6d0f0383e2a8f243f439828ec`; `SC1-6dcae013f3573ea6a101d0f52a15bf98a8a0c9f7c2fb54ce37037bd6d3ab8c4c` / `SC1-0b12eab538b2ee0246dacf41759af4c4adc0f84639b9a803e919cc7c47e64b2a`; `SC1-cc866d2c54474581222124f08e4771ee92fe3dc9ecd488feb8516b4f101aa62f` / `SC1-2798a31ed0ef54159a8e043093e95814eca56ff8285a41eb4751a4af0940fa2e`; `SC1-646376386dc069be0fa83c0a35c3659ff40106fafb80490684b32b0f78915315` / `SC1-aa21ceb83704d525863bb64b73066198d5f1842613a0d689c06f804107588c35`; `SC1-fe03c6471a4644a842d2a949977a40ad0a7fecae11f15ff69344aaec4870f080` / `SC1-96e2f072d676a8b31dad14b1a26ddcc7043833c1387bfde75ea72e3a4025109f`; `SC1-d97942f65db3c01c6bcb8cc20664e753f71ab4fe25efd396d3fc02880ef36a62` / `SC1-ad316889cee0a5fb23c7b2fdbd65a1641e95634d4597fc1bef69c8ad1b2fcfba`; `SC1-1562b13372233a0257c9f6df1146ca68024b2c9ecb52c467f3b973265a239663` / `SC1-68a51cac4411459a3f08b77df58205cc490a34243d94f7a4a519c597195f5506`.
- X86_64 guard/conditionals: `SC1-b6712c8e985ca1bfcc59ac3167798a493bebb6090c79a77df0c6dd527db22a2a`, `SC1-e610cc33c6289f24e3d80bc661f3d6eca0e1f66bed8bb13be5f590413ab13bd3`, `SC1-e477e68bd6b43d4dac8a151a4d7f4de2b632ceead2f524e2887e121a8deec0af`, `SC1-c761a72ed027e0f495830594bb58c0f0bdab09fca882b9c0693921ab19d26e12`.
- X86_64 macros in the same source order, selection then status: `SC1-7361ea87a353eae15995ccee31e2c6accbb45a614e64e62654ff3207c895190c` / `SC1-ae238333e6f1646d2cec614663a70b18ede4020017a2a0f502e1f0f719ee26fd`; `SC1-5fef48df6524e6034465077f49fa8689087f476d562a8111f618c621a0502043` / `SC1-7baab86e06791e3c98ee87d83e2f5c8fb3de1b469997f45a40c1304a18e94a74`; `SC1-d841ce71b014fe21527abb7966645fc093d3feef0757e68f137d863b99eaa37f` / `SC1-cf4967c260f386a075c7cdc7fee9c29ae5a28475e996e86f6af3e044814be97e`; `SC1-7f25a6dde107ff12c3fe3deb5e4ccd65cd065166f6c1b16378d9a0fdb253db12` / `SC1-b4848a369ccfebb47c6bc804a2edb9be1f73965d2f6b6f35fbbebcd2608240e4`; `SC1-5dc93d265e3924d00f11fe06f6f74fe1c201600db236ef7ade12e45bc9215ad8` / `SC1-4a8773def8baea8b66a2a5e8390ebb1b6581d26d380fe171116d330535526423`; `SC1-d8192c410be56eb1aad811ffdd472d96e2d3cd38ededd23a8515732d0587aed1` / `SC1-5817f0a3e6984d0f3a3a2b0ed1e567f2f5127e5d84b105a55169be00c4d08613`; `SC1-9334d5df95ba7303a364dbab532da6ec81a23f89d5360167c6c7f75b07cc1e00` / `SC1-67ced90ac54cc39f05c0f4cbc7956040f2a6ded7832f99f2e09450f4eb793be0`; `SC1-cad358612fc5c0f58a089b93ba5d0956ab1560b1e6e272991e4a8a238771ac43` / `SC1-c8e37ce590c238a513ffd110272829d68e170617360a93ef932efdc01ce7fb94`; `SC1-b3019916b26f1398de199a2b0221875988ab61a98b80ed0e9b778dd9484b8855` / `SC1-12d56d1b063bb5f68fda1ea7c34cf8de71df619b25ca401e4ed0ee32602632df`; `SC1-90dde0db729f2232098454e65f1199dfc17baa977a60409ab6392a0f1d7845f9` / `SC1-3d5fa9b6cfc2af6f35d32e74ef4e7482465f3317256d5c0c2926ea079f1548ba`; `SC1-e77da849f548c793774805a4ad935bd4e1fe050efff0e4d67be2a13173239dd5` / `SC1-0b7fc78b8263d44d89ec431b29cd43923c7b8325f0448f8f62d38885c73ad6b8`; `SC1-f7f864945ec82a8db0679efe294a08bfe6229937ca6430d163b799c33123fbb5` / `SC1-865a3c7b4462e5ec274ac07dc957bdb2b2797ad6fa7aa309c27827453c27c9f5`; `SC1-7c75d9bd7e64dbdc60c3a6f6fab69225de4b06b0f7750d286eb3efbfe13afa6f` / `SC1-bf2baf2d348f3a0701d0e0ca1076e9111c41ed28ba2ed74b438e3e0a287465f1`; `SC1-c2ecf9adc93c99c1f720fb46e2f6c348c56094dc9a09ce8873761b1c7e6bd087` / `SC1-d3b3cf5a31055b055c9cc97003a15a4d34eabbf444efe0407925740d3d23c7b3`; `SC1-9435d5179c427b4af28e297b9e0286ade4f6016582161931a194dfdbc8ead88e` / `SC1-a4dd8ad49a684e70a9b8374551dd268f55a0f1f3a8013c0fbe29d96b9f29d14a`.

## Result

**REJECT — one finding (P1).**  The numeric UAPI mechanism and its direct NFS
uses otherwise match the pinned source.  The applier must restore the notice
before final acceptance.
