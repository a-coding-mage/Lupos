# Parity review — S016277, attempt 3, slot 1

Result: **REJECT**. Manual source review only; no compiler, formatter, test, rust-analyzer diagnostic, or historical Lupos source was used.

Candidate: src/include/uapi/linux/netfilter/nf_tables.rs.
Pinned oracle: vendor/linux/include/uapi/linux/netfilter/nf_tables.h at 425f94c2954b1fe80ebdbf9b29854e89750355df.
Current sealed proposal: 709625d12a62d076e2bfba312a2d11566dfaf6515c1587c4267424b778851447 (5,737 records), candidate digest f9bc404a3e57f6ebbd09d763cce679b80cfbac989b82c4bbf9752531370d87cc, Phase-0 identity 0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2, queue fingerprint cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f.

## Findings

1. **Unauthorized SPDX delta.** Linux line 1 is GPL-2.0 WITH Linux-syscall-note; candidate line 1 changes it to GPL-2.0-only WITH Linux-syscall-note. These are different SPDX expressions and rewrite/BRANDING_ALLOWLIST.tsv has no entry. Closure key: SC1-7891aee3610f34ec984fc67dc1794d5a5d477b7c3dab2e557a6fa740fa5346e2 (SCOPE.tsv:16278).

2. **Guard semantics are omitted.** Linux lines 2--3 and 2022 establish _LINUX_NF_TABLES_H; candidate has no equivalent or mapping. Closure keys: aarch64 SC1-0591e6ab06835986db6582921f1455deea2048fac4921cd843805c3cb556be68 (SYMBOLS.tsv:385334); x86_64 SC1-f41cd7fe8058915253084148e57aa6908ab45c160dc13bb0f61d5f45b09fd1fd (SYMBOLS.tsv:386301).

3. **Selected symbol NFT_REG32_MAX is absent.** Linux lines 49--51 define it as NFT_REG32_15 under __KERNEL__; candidate has no declaration. Both frozen kernel/auditsc.o compile commands contain -D__KERNEL__, and rewrite/metadata/header_closure.tsv records this as a selected header consumer. Closure keys: aarch64 SC1-56e19584fe02938c2f1058ca04fee0f14dbb50fb05106692edd7d0de48c7f28d (SYMBOLS.tsv:385347); x86_64 SC1-69eeb13964c5549ca8fe8e3311a4dde4456a8a0b8f76139a097864834ee812de (SYMBOLS.tsv:386314).

4. **Selected macro expressions use undeclared, misspelled operands.** Every candidate macro below loses the upstream expression/value; this includes Linux continuation-backed macros at lines 194, 225, and 1979.

| Linux symbol:line | Candidate line / bad operand | aarch64 closure key | x86_64 closure key |
| --- | --- | --- | --- |
| NFT_TABLE_F_MASK:194 | 194 / NFT_TABE_F_* | SC1-aaa7874489bdbb88d7ee0272c8ff9aa0783483c889bdebd987befe15c1b838c2 | SC1-71908ae8a35b67e990cd1d6028a1854327f5c801a015021e9ad3391daee1e01b |
| NFTA_TABLE_MAX:218 | 216 / __NFTA_TABE_MAX | SC1-ff52d37efd393fab152c807089bbb7397c7ba1fa368e26bc63bc01247ac427fc | SC1-f3d71813cd2199ca80f8b5432d762c72e8b1403b5ea81412edea629baa8b3309 |
| NFT_CHAIN_FLAGS:225 | 223 / NFT_CHAIN_HW_OFFOAD | SC1-e8f7cf0fa867abcaa6115af2774651af5c158a22b6066acbef88128f704e7185 | SC1-97153dc8d4d6643990458f41488950176ff822ce046cdf64fe413643fd61f732 |
| NFTA_SET_FIELD_MAX:379 | 375 / __NFTA_SET_FIED_MAX | SC1-175ac0e81f8090b56dddbe4f9fdc266c1cd2866c2da465ca6429327f9a64c224 | SC1-871949795ef8bbd65fddaf9c7a2428fa1b8ef08adbc6544536a0ee2b4b75a74c |
| NFTA_SET_ELEM_MAX:470 | 466 / __NFTA_SET_EEM_MAX | SC1-dbe4c1cdc29148b7ed72e4bb0c7eb0ca509eaa68b54da9991bf8b207d767502d | SC1-1dc8b2084b189aa4bdf275be6d955e3bdead905cc8e559eeb651073121fd3a26 |
| NFTA_SET_ELEM_LIST_MAX:488 | 484 / __NFTA_SET_EEM_LIST_MAX | SC1-f2fc9b974a13239f2ea00082d42f0a0995e0c3de7ca2d79471801fa3b7215deb | SC1-692b37e60ea1f0a664d702e9db4282b7a021bed9f9fd1c6b76a96fd5403fff8a |
| NFTA_FLOW_MAX:1215 | 1210 / __NFTA_FOW_MAX | SC1-c37836ced8b92f3ab6ccc7e25a54f20889272a9078ce3b8fb5aaa90bc77afd94 | SC1-1c18eec7e3e41de239bac352b872b32d319420f24f292a4e39fee17f073b8be6 |
| NFT_LOGLEVEL_MAX:1342 | 1337 / __NFT_LOGLEVE_MAX | SC1-281205a0fe9b441a186e521f3742c89e8808ad529a6ecd51e97c484b1a91a5b0 | SC1-57c2a5f7d320e324320cd14bd5e522069816bf8fe2175696b4a30ae23be9260e |
| NFTA_QUEUE_MAX:1360 | 1355 / __NFTA_QUEE_MAX | SC1-00f0d285d5010a9a83163e2f4e1125a29384ec7becdc15d38ad9457fe234772b | SC1-a2da7c2839a9f2307602cddc4217a3a798ec4459f4cc94e4064f8225ad831c99 |
| NFTA_DUP_MAX:1544 | 1539 / __NFTA_DP_MAX | SC1-00d48aedf1a850ff84ae41bcea63491600893de22b510dc3ab26aa18daaa1504 | SC1-ec1c4e870a434b2091847cad781801644a19171acafd6fbd1a7cbf6f039f8189 |
| NFTA_CT_HELPER_MAX:1640 | 1635 / __NFTA_CT_HEPER_MAX | SC1-d1d8b6a5a6a10a8ada4c9bb1f497183bc8052ccf784d811ab1e4d13740767fdd | SC1-21c2d55949ffd2502d04d4b8f3799862819cdf281caa30b28831d73970f55ec9 |
| NFTA_FLOWTABLE_MAX:1735 | 1729 / __NFTA_FOWTABE_MAX | SC1-7d5bd57b63f8c73eaf336c4f463e819283ea780b5f12b70045105b0358abe513 | SC1-72a631b7c45123ab0b2382c9cf3b9e64062d5cd83758c3868d0fa2c741a5a6ea |
| NFTA_FLOWTABLE_HOOK_MAX:1751 | 1745 / __NFTA_FOWTABE_HOOK_MAX | SC1-96f5ff6c458368d66956978e68d2d7e1ade004765056d31e737651b6ab616488 | SC1-d6721847f4b7894d0f14fb2546dd05c5375f6fae6d37aacbc17a522e10456460 |
| NFTA_TUNNEL_KEY_IP_MAX:1928 | 1922 / __NFTA_TUNNE_KEY_IP_MAX | SC1-dd3a91bf6ff0ae8c858aebb854bd830dcacf79197143d208457df608c4db7158 | SC1-28696adc524fd13ff834872eaee2af07135b9874cf82f065559623537b4fa17b |
| NFTA_TUNNEL_KEY_IP6_MAX:1937 | 1931 / __NFTA_TUNNE_KEY_IP6_MAX | SC1-1cca8fd00c66a1655e0c180c11c7cea06d1721d099165c42937eb3a91eb4049e | SC1-03540db437ee2bbf2a4295b734c5c358e05df85bb68f10b44bf97b5b81e55068 |
| NFTA_TUNNEL_KEY_OPTS_MAX:1946 | 1940 / __NFTA_TUNNE_KEY_OPTS_MAX | SC1-cb8a1e3fa9616c18d36ff504003ff025ee61971b1e524a28c724edbfc96e35ee | SC1-77f3b41533138e290251ce3c9b06fe831b9f61050a74d2519d00bc0ebbabdfb5 |
| NFTA_TUNNEL_KEY_VXLAN_MAX:1953 | 1947 / __NFTA_TUNNE_KEY_VXLAN_MAX | SC1-085d87ed06ea91c23ac3ddc6b07aff9efad9e54e66579ea53139df915c6cf19c | SC1-e6d3e7afc85648c2132737dcf9e734fdb76d04096a40d906743dedd5c1e3669e |
| NFTA_TUNNEL_KEY_ERSPAN_MAX:1963 | 1957 / __NFTA_TUNNE_KEY_ERSPAN_MAX | SC1-2d8246ae50b9346dbbf3750f6c6de6bccf4fc7372f7e105b9082eb3f36e006ec | SC1-149883948fee9e6c6ebb7f9ef5bf17975d825b1e24a4abe9cc42e24fae96d75b |
| NFTA_TUNNEL_KEY_GENEVE_MAX:1972 | 1966 / __NFTA_TUNNE_KEY_GENEVE_MAX | SC1-3ee1cb17ad9c1512d8f96d53a2b2e774ab11f00b6406dee66f26a0ed14de4ac4 | SC1-8cd746be91e6afa4252d7bc12c9abdf360fc7fea7bb148ea15967515dc75150b |
| NFT_TUNNEL_F_MASK:1979 | 1973 / NFT_TUNNE_F_* | SC1-64ef2737143e451ab35cb4ad1662bf0e7e8893f50a863870d245927b48fc425d | SC1-c522ae2823ad5c7d623d698fc9bc8e64441b8baf4cc3581acd41d50f6f073d71 |
| NFTA_TUNNEL_KEY_MAX:1996 | 1988 / __NFTA_TUNNE_KEY_MAX | SC1-547cdf1c792ec1f582d1669e1fe81c7280490a1f7e227ac881d4f221c87497f4 | SC1-eb8c89943e6a421f07840f2e4a7f96d39fff0a1d186fa8833da2bbfdbe3df9bb |
| NFT_TUNNEL_MAX:2003 | 1995 / __NFT_TUNNE_MAX | SC1-f40673dce4654f218f0cf20aca5c30d21ebaa4dde2a92d8ef518ff65b84546c5 | SC1-4d6f4e532c2ed4c5134fdb432e66ffe1795871b99336113ef6a93cef51afc5ec |
| NFT_TUNNEL_MODE_MAX:2011 | 2003 / __NFT_TUNNE_MODE_MAX | SC1-dc2b5de323367d2f95f61f65a27ec94b5ad171acd99262efeb96cb1b17a1ad79 | SC1-81464c3d924db4bb7acb361b7f96f2620bc23162fe5286ff1f2c2cb2eebb95b6 |
| NFTA_TUNNEL_MAX:2020 | 2012 / __NFTA_TUNNE_MAX | SC1-fb62c3b580a7f6369e5af0b58619f628830dc52cac9d2c3dc4a1ba1d5141a292 | SC1-526436558744767a69929c905206047ef7f1fafb195ddff6733a20e0ceade472 |

5. **Enum ABI remains unclosed.** Candidate scalar aliases (i32, with nft_data_types = u32) do not establish C enum representation/signedness. The frozen ABI records are PENDING_REVIEW, but the proposal marks 230 ABI type/status mappings COMPLETE (115 per architecture). This is a source-review blocker. Representative exact closure keys: enum nft_registers, aarch64 SC1-1d333e221ca4a39297c89fdd95e47456c98e68e945a28ec46a6ba2ae6262ffee, x86_64 SC1-2f0060d7aa169c8bef0a7e1e3e5efc6d34f122556b41da82229b2ab91d0e8628; enum nft_data_types, aarch64 SC1-0be697b62c8a40dff36476b08226a596e4e6c54d73c81054e8040573be17db4d, x86_64 SC1-04051c2c6b92ad260d7490dda6f2fe7067acda62bff09ed0ddeacf6cd1259ce4. Local source: Linux lines 22 and 504; ABI rows 193137/193252 and 193157/193272.

All remaining names and enum ordering were manually compared to the pinned header. These findings prevent semantic closure acceptance.

