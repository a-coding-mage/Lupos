// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: include/uapi/linux/snmp.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: common
//! rewrite-task: S016384

/* SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note */
/*
 * Definitions for MIBs
 *
 * Author: Hideaki YOSHIFUJI <yoshfuji@linux-ipv6.org>
 */


/* ipstats mib definitions */
/*
 * RFC 1213:  MIB-II
 * RFC 2011 (updates 1213):  SNMPv2-MIB-IP
 * RFC 2863:  Interfaces Group MIB
 * RFC 2465:  IPv6 MIB: General Group
 * draft-ietf-ipv6-rfc2011-update-10.txt: MIB for IP: IP Statistics Tables
 */
// C anonymous enum translated as i32 constants
pub const IPSTATS_MIB_NUM: i32 = 0;
/* frequently written fields in fast path, kept in same cache line */
pub const IPSTATS_MIB_INPKTS: i32 = 1; /* InReceives */
pub const IPSTATS_MIB_INOCTETS: i32 = 2; /* InOctets */
pub const IPSTATS_MIB_INDELIVERS: i32 = 3; /* InDelivers */
pub const IPSTATS_MIB_NOECTPKTS: i32 = 4; /* InNoECTPkts */
pub const IPSTATS_MIB_ECT1PKTS: i32 = 5; /* InECT1Pkts */
pub const IPSTATS_MIB_ECT0PKTS: i32 = 6; /* InECT0Pkts */
pub const IPSTATS_MIB_CEPKTS: i32 = 7; /* InCEPkts */
pub const IPSTATS_MIB_OUTREQUESTS: i32 = 8; /* OutRequests */
pub const IPSTATS_MIB_OUTPKTS: i32 = 9; /* OutTransmits */
pub const IPSTATS_MIB_OUTOCTETS: i32 = 10; /* OutOctets */
pub const IPSTATS_MIB_OUTFORWDATAGRAMS: i32 = 11; /* OutForwDatagrams */
/* other fields */
pub const IPSTATS_MIB_INHDRERRORS: i32 = 12; /* InHdrErrors */
pub const IPSTATS_MIB_INTOOBIGERRORS: i32 = 13; /* InTooBigErrors */
pub const IPSTATS_MIB_INNOROUTES: i32 = 14; /* InNoRoutes */
pub const IPSTATS_MIB_INADDRERRORS: i32 = 15; /* InAddrErrors */
pub const IPSTATS_MIB_INUNKNOWNPROTOS: i32 = 16; /* InUnknownProtos */
pub const IPSTATS_MIB_INTRUNCATEDPKTS: i32 = 17; /* InTruncatedPkts */
pub const IPSTATS_MIB_INDISCARDS: i32 = 18; /* InDiscards */
pub const IPSTATS_MIB_OUTDISCARDS: i32 = 19; /* OutDiscards */
pub const IPSTATS_MIB_OUTNOROUTES: i32 = 20; /* OutNoRoutes */
pub const IPSTATS_MIB_REASMTIMEOUT: i32 = 21; /* ReasmTimeout */
pub const IPSTATS_MIB_REASMREQDS: i32 = 22; /* ReasmReqds */
pub const IPSTATS_MIB_REASMOKS: i32 = 23; /* ReasmOKs */
pub const IPSTATS_MIB_REASMFAILS: i32 = 24; /* ReasmFails */
pub const IPSTATS_MIB_FRAGOKS: i32 = 25; /* FragOKs */
pub const IPSTATS_MIB_FRAGFAILS: i32 = 26; /* FragFails */
pub const IPSTATS_MIB_FRAGCREATES: i32 = 27; /* FragCreates */
pub const IPSTATS_MIB_INMCASTPKTS: i32 = 28; /* InMcastPkts */
pub const IPSTATS_MIB_OUTMCASTPKTS: i32 = 29; /* OutMcastPkts */
pub const IPSTATS_MIB_INBCASTPKTS: i32 = 30; /* InBcastPkts */
pub const IPSTATS_MIB_OUTBCASTPKTS: i32 = 31; /* OutBcastPkts */
pub const IPSTATS_MIB_INMCASTOCTETS: i32 = 32; /* InMcastOctets */
pub const IPSTATS_MIB_OUTMCASTOCTETS: i32 = 33; /* OutMcastOctets */
pub const IPSTATS_MIB_INBCASTOCTETS: i32 = 34; /* InBcastOctets */
pub const IPSTATS_MIB_OUTBCASTOCTETS: i32 = 35; /* OutBcastOctets */
pub const IPSTATS_MIB_CSUMERRORS: i32 = 36; /* InCsumErrors */
pub const IPSTATS_MIB_REASM_OVERLAPS: i32 = 37; /* ReasmOverlaps */
pub const __IPSTATS_MIB_MAX: i32 = 38;


/* icmp mib definitions */
/*
 * RFC 1213:  MIB-II ICMP Group
 * RFC 2011 (updates 1213):  SNMPv2 MIB for IP: ICMP group
 */
// C anonymous enum translated as i32 constants
pub const ICMP_MIB_NUM: i32 = 0;
pub const ICMP_MIB_INMSGS: i32 = 1; /* InMsgs */
pub const ICMP_MIB_INERRORS: i32 = 2; /* InErrors */
pub const ICMP_MIB_INDESTUNREACHS: i32 = 3; /* InDestUnreachs */
pub const ICMP_MIB_INTIMEEXCDS: i32 = 4; /* InTimeExcds */
pub const ICMP_MIB_INPARMPROBS: i32 = 5; /* InParmProbs */
pub const ICMP_MIB_INSRCQUENCHS: i32 = 6; /* InSrcQuenchs */
pub const ICMP_MIB_INREDIRECTS: i32 = 7; /* InRedirects */
pub const ICMP_MIB_INECHOS: i32 = 8; /* InEchos */
pub const ICMP_MIB_INECHOREPS: i32 = 9; /* InEchoReps */
pub const ICMP_MIB_INTIMESTAMPS: i32 = 10; /* InTimestamps */
pub const ICMP_MIB_INTIMESTAMPREPS: i32 = 11; /* InTimestampReps */
pub const ICMP_MIB_INADDRMASKS: i32 = 12; /* InAddrMasks */
pub const ICMP_MIB_INADDRMASKREPS: i32 = 13; /* InAddrMaskReps */
pub const ICMP_MIB_OUTMSGS: i32 = 14; /* OutMsgs */
pub const ICMP_MIB_OUTERRORS: i32 = 15; /* OutErrors */
pub const ICMP_MIB_OUTDESTUNREACHS: i32 = 16; /* OutDestUnreachs */
pub const ICMP_MIB_OUTTIMEEXCDS: i32 = 17; /* OutTimeExcds */
pub const ICMP_MIB_OUTPARMPROBS: i32 = 18; /* OutParmProbs */
pub const ICMP_MIB_OUTSRCQUENCHS: i32 = 19; /* OutSrcQuenchs */
pub const ICMP_MIB_OUTREDIRECTS: i32 = 20; /* OutRedirects */
pub const ICMP_MIB_OUTECHOS: i32 = 21; /* OutEchos */
pub const ICMP_MIB_OUTECHOREPS: i32 = 22; /* OutEchoReps */
pub const ICMP_MIB_OUTTIMESTAMPS: i32 = 23; /* OutTimestamps */
pub const ICMP_MIB_OUTTIMESTAMPREPS: i32 = 24; /* OutTimestampReps */
pub const ICMP_MIB_OUTADDRMASKS: i32 = 25; /* OutAddrMasks */
pub const ICMP_MIB_OUTADDRMASKREPS: i32 = 26; /* OutAddrMaskReps */
pub const ICMP_MIB_CSUMERRORS: i32 = 27; /* InCsumErrors */
pub const ICMP_MIB_RATELIMITGLOBAL: i32 = 28; /* OutRateLimitGlobal */
pub const ICMP_MIB_RATELIMITHOST: i32 = 29; /* OutRateLimitHost */
pub const __ICMP_MIB_MAX: i32 = 30;


pub const __ICMPMSG_MIB_MAX: i32 = 512; /* Out+In for all 8-bit ICMP types */

/* icmp6 mib definitions */
/*
 * RFC 2466:  ICMPv6-MIB
 */
// C anonymous enum translated as i32 constants
pub const ICMP6_MIB_NUM: i32 = 0;
pub const ICMP6_MIB_INMSGS: i32 = 1; /* InMsgs */
pub const ICMP6_MIB_INERRORS: i32 = 2; /* InErrors */
pub const ICMP6_MIB_OUTMSGS: i32 = 3; /* OutMsgs */
pub const ICMP6_MIB_OUTERRORS: i32 = 4; /* OutErrors */
pub const ICMP6_MIB_CSUMERRORS: i32 = 5; /* InCsumErrors */
pub const ICMP6_MIB_RATELIMITHOST: i32 = 6; /* OutRateLimitHost */
pub const __ICMP6_MIB_MAX: i32 = 7;


pub const __ICMP6MSG_MIB_MAX: i32 = 512; /* Out+In for all 8-bit ICMPv6 types */

/* tcp mib definitions */
/*
 * RFC 1213:  MIB-II TCP group
 * RFC 2012 (updates 1213):  SNMPv2-MIB-TCP
 */
// C anonymous enum translated as i32 constants
pub const TCP_MIB_NUM: i32 = 0;
pub const TCP_MIB_RTOALGORITHM: i32 = 1; /* RtoAlgorithm */
pub const TCP_MIB_RTOMIN: i32 = 2; /* RtoMin */
pub const TCP_MIB_RTOMAX: i32 = 3; /* RtoMax */
pub const TCP_MIB_MAXCONN: i32 = 4; /* MaxConn */
pub const TCP_MIB_ACTIVEOPENS: i32 = 5; /* ActiveOpens */
pub const TCP_MIB_PASSIVEOPENS: i32 = 6; /* PassiveOpens */
pub const TCP_MIB_ATTEMPTFAILS: i32 = 7; /* AttemptFails */
pub const TCP_MIB_ESTABRESETS: i32 = 8; /* EstabResets */
pub const TCP_MIB_CURRESTAB: i32 = 9; /* CurrEstab */
pub const TCP_MIB_INSEGS: i32 = 10; /* InSegs */
pub const TCP_MIB_OUTSEGS: i32 = 11; /* OutSegs */
pub const TCP_MIB_RETRANSSEGS: i32 = 12; /* RetransSegs */
pub const TCP_MIB_INERRS: i32 = 13; /* InErrs */
pub const TCP_MIB_OUTRSTS: i32 = 14; /* OutRsts */
pub const TCP_MIB_CSUMERRORS: i32 = 15; /* InCsumErrors */
pub const __TCP_MIB_MAX: i32 = 16;


/* udp mib definitions */
/*
 * RFC 1213:  MIB-II UDP group
 * RFC 2013 (updates 1213):  SNMPv2-MIB-UDP
 */
// C anonymous enum translated as i32 constants
pub const UDP_MIB_NUM: i32 = 0;
pub const UDP_MIB_INDATAGRAMS: i32 = 1; /* InDatagrams */
pub const UDP_MIB_NOPORTS: i32 = 2; /* NoPorts */
pub const UDP_MIB_INERRORS: i32 = 3; /* InErrors */
pub const UDP_MIB_OUTDATAGRAMS: i32 = 4; /* OutDatagrams */
pub const UDP_MIB_RCVBUFERRORS: i32 = 5; /* RcvbufErrors */
pub const UDP_MIB_SNDBUFERRORS: i32 = 6; /* SndbufErrors */
pub const UDP_MIB_CSUMERRORS: i32 = 7; /* InCsumErrors */
pub const UDP_MIB_IGNOREDMULTI: i32 = 8; /* IgnoredMulti */
pub const UDP_MIB_MEMERRORS: i32 = 9; /* MemErrors */
pub const __UDP_MIB_MAX: i32 = 10;


/* linux mib definitions */
// C anonymous enum translated as i32 constants
pub const LINUX_MIB_NUM: i32 = 0;
pub const LINUX_MIB_SYNCOOKIESSENT: i32 = 1; /* SyncookiesSent */
pub const LINUX_MIB_SYNCOOKIESRECV: i32 = 2; /* SyncookiesRecv */
pub const LINUX_MIB_SYNCOOKIESFAILED: i32 = 3; /* SyncookiesFailed */
pub const LINUX_MIB_EMBRYONICRSTS: i32 = 4; /* EmbryonicRsts */
pub const LINUX_MIB_PRUNECALLED: i32 = 5; /* PruneCalled */
pub const LINUX_MIB_RCVPRUNED: i32 = 6; /* RcvPruned */
pub const LINUX_MIB_OFOPRUNED: i32 = 7; /* OfoPruned */
pub const LINUX_MIB_OUTOFWINDOWICMPS: i32 = 8; /* OutOfWindowIcmps */
pub const LINUX_MIB_LOCKDROPPEDICMPS: i32 = 9; /* LockDroppedIcmps */
pub const LINUX_MIB_ARPFILTER: i32 = 10; /* ArpFilter */
pub const LINUX_MIB_TIMEWAITED: i32 = 11; /* TimeWaited */
pub const LINUX_MIB_TIMEWAITRECYCLED: i32 = 12; /* TimeWaitRecycled */
pub const LINUX_MIB_TIMEWAITKILLED: i32 = 13; /* TimeWaitKilled */
pub const LINUX_MIB_PAWSACTIVEREJECTED: i32 = 14; /* PAWSActiveRejected */
pub const LINUX_MIB_PAWSESTABREJECTED: i32 = 15; /* PAWSEstabRejected */
pub const LINUX_MIB_BEYOND_WINDOW: i32 = 16; /* BeyondWindow */
pub const LINUX_MIB_TSECRREJECTED: i32 = 17; /* TSEcrRejected */
pub const LINUX_MIB_PAWS_OLD_ACK: i32 = 18; /* PAWSOldAck */
pub const LINUX_MIB_PAWS_TW_REJECTED: i32 = 19; /* PAWSTimewait */
pub const LINUX_MIB_DELAYEDACKS: i32 = 20; /* DelayedACKs */
pub const LINUX_MIB_DELAYEDACKLOCKED: i32 = 21; /* DelayedACKLocked */
pub const LINUX_MIB_DELAYEDACKLOST: i32 = 22; /* DelayedACKLost */
pub const LINUX_MIB_LISTENOVERFLOWS: i32 = 23; /* ListenOverflows */
pub const LINUX_MIB_LISTENDROPS: i32 = 24; /* ListenDrops */
pub const LINUX_MIB_TCPHPHITS: i32 = 25; /* TCPHPHits */
pub const LINUX_MIB_TCPPUREACKS: i32 = 26; /* TCPPureAcks */
pub const LINUX_MIB_TCPHPACKS: i32 = 27; /* TCPHPAcks */
pub const LINUX_MIB_TCPRENORECOVERY: i32 = 28; /* TCPRenoRecovery */
pub const LINUX_MIB_TCPSACKRECOVERY: i32 = 29; /* TCPSackRecovery */
pub const LINUX_MIB_TCPSACKRENEGING: i32 = 30; /* TCPSACKReneging */
pub const LINUX_MIB_TCPSACKREORDER: i32 = 31; /* TCPSACKReorder */
pub const LINUX_MIB_TCPRENOREORDER: i32 = 32; /* TCPRenoReorder */
pub const LINUX_MIB_TCPTSREORDER: i32 = 33; /* TCPTSReorder */
pub const LINUX_MIB_TCPFULLUNDO: i32 = 34; /* TCPFullUndo */
pub const LINUX_MIB_TCPPARTIALUNDO: i32 = 35; /* TCPPartialUndo */
pub const LINUX_MIB_TCPDSACKUNDO: i32 = 36; /* TCPDSACKUndo */
pub const LINUX_MIB_TCPLOSSUNDO: i32 = 37; /* TCPLossUndo */
pub const LINUX_MIB_TCPLOSTRETRANSMIT: i32 = 38; /* TCPLostRetransmit */
pub const LINUX_MIB_TCPRENOFAILURES: i32 = 39; /* TCPRenoFailures */
pub const LINUX_MIB_TCPSACKFAILURES: i32 = 40; /* TCPSackFailures */
pub const LINUX_MIB_TCPLOSSFAILURES: i32 = 41; /* TCPLossFailures */
pub const LINUX_MIB_TCPFASTRETRANS: i32 = 42; /* TCPFastRetrans */
pub const LINUX_MIB_TCPSLOWSTARTRETRANS: i32 = 43; /* TCPSlowStartRetrans */
pub const LINUX_MIB_TCPTIMEOUTS: i32 = 44; /* TCPTimeouts */
pub const LINUX_MIB_TCPLOSSPROBES: i32 = 45; /* TCPLossProbes */
pub const LINUX_MIB_TCPLOSSPROBERECOVERY: i32 = 46; /* TCPLossProbeRecovery */
pub const LINUX_MIB_TCPRENORECOVERYFAIL: i32 = 47; /* TCPRenoRecoveryFail */
pub const LINUX_MIB_TCPSACKRECOVERYFAIL: i32 = 48; /* TCPSackRecoveryFail */
pub const LINUX_MIB_TCPRCVCOLLAPSED: i32 = 49; /* TCPRcvCollapsed */
pub const LINUX_MIB_TCPDSACKOLDSENT: i32 = 50; /* TCPDSACKOldSent */
pub const LINUX_MIB_TCPDSACKOFOSENT: i32 = 51; /* TCPDSACKOfoSent */
pub const LINUX_MIB_TCPDSACKRECV: i32 = 52; /* TCPDSACKRecv */
pub const LINUX_MIB_TCPDSACKOFORECV: i32 = 53; /* TCPDSACKOfoRecv */
pub const LINUX_MIB_TCPABORTONDATA: i32 = 54; /* TCPAbortOnData */
pub const LINUX_MIB_TCPABORTONCLOSE: i32 = 55; /* TCPAbortOnClose */
pub const LINUX_MIB_TCPABORTONMEMORY: i32 = 56; /* TCPAbortOnMemory */
pub const LINUX_MIB_TCPABORTONTIMEOUT: i32 = 57; /* TCPAbortOnTimeout */
pub const LINUX_MIB_TCPABORTONLINGER: i32 = 58; /* TCPAbortOnLinger */
pub const LINUX_MIB_TCPABORTFAILED: i32 = 59; /* TCPAbortFailed */
pub const LINUX_MIB_TCPMEMORYPRESSURES: i32 = 60; /* TCPMemoryPressures */
pub const LINUX_MIB_TCPMEMORYPRESSURESCHRONO: i32 = 61; /* TCPMemoryPressuresChrono */
pub const LINUX_MIB_TCPSACKDISCARD: i32 = 62; /* TCPSACKDiscard */
pub const LINUX_MIB_TCPDSACKIGNOREDOLD: i32 = 63; /* TCPSACKIgnoredOld */
pub const LINUX_MIB_TCPDSACKIGNOREDNOUNDO: i32 = 64; /* TCPSACKIgnoredNoUndo */
pub const LINUX_MIB_TCPSPURIOUSRTOS: i32 = 65; /* TCPSpuriousRTOs */
pub const LINUX_MIB_TCPMD5NOTFOUND: i32 = 66; /* TCPMD5NotFound */
pub const LINUX_MIB_TCPMD5UNEXPECTED: i32 = 67; /* TCPMD5Unexpected */
pub const LINUX_MIB_TCPMD5FAILURE: i32 = 68; /* TCPMD5Failure */
pub const LINUX_MIB_SACKSHIFTED: i32 = 69;
pub const LINUX_MIB_SACKMERGED: i32 = 70;
pub const LINUX_MIB_SACKSHIFTFALLBACK: i32 = 71;
pub const LINUX_MIB_TCPBACKLOGDROP: i32 = 72;
pub const LINUX_MIB_PFMEMALLOCDROP: i32 = 73;
pub const LINUX_MIB_TCPMINTTLDROP: i32 = 74; /* RFC 5082 */
pub const LINUX_MIB_TCPDEFERACCEPTDROP: i32 = 75;
pub const LINUX_MIB_IPRPFILTER: i32 = 76; /* IP Reverse Path Filter (rp_filter) */
pub const LINUX_MIB_TCPTIMEWAITOVERFLOW: i32 = 77; /* TCPTimeWaitOverflow */
pub const LINUX_MIB_TCPREQQFULLDOCOOKIES: i32 = 78; /* TCPReqQFullDoCookies */
pub const LINUX_MIB_TCPREQQFULLDROP: i32 = 79; /* TCPReqQFullDrop */
pub const LINUX_MIB_TCPRETRANSFAIL: i32 = 80; /* TCPRetransFail */
pub const LINUX_MIB_TCPRCVCOALESCE: i32 = 81; /* TCPRcvCoalesce */
pub const LINUX_MIB_TCPBACKLOGCOALESCE: i32 = 82; /* TCPBacklogCoalesce */
pub const LINUX_MIB_TCPOFOQUEUE: i32 = 83; /* TCPOFOQueue */
pub const LINUX_MIB_TCPOFODROP: i32 = 84; /* TCPOFODrop */
pub const LINUX_MIB_TCPOFOMERGE: i32 = 85; /* TCPOFOMerge */
pub const LINUX_MIB_TCPCHALLENGEACK: i32 = 86; /* TCPChallengeACK */
pub const LINUX_MIB_TCPSYNCHALLENGE: i32 = 87; /* TCPSYNChallenge */
pub const LINUX_MIB_TCPFASTOPENACTIVE: i32 = 88; /* TCPFastOpenActive */
pub const LINUX_MIB_TCPFASTOPENACTIVEFAIL: i32 = 89; /* TCPFastOpenActiveFail */
pub const LINUX_MIB_TCPFASTOPENPASSIVE: i32 = 90; /* TCPFastOpenPassive*/
pub const LINUX_MIB_TCPFASTOPENPASSIVEFAIL: i32 = 91; /* TCPFastOpenPassiveFail */
pub const LINUX_MIB_TCPFASTOPENLISTENOVERFLOW: i32 = 92; /* TCPFastOpenListenOverflow */
pub const LINUX_MIB_TCPFASTOPENCOOKIEREQD: i32 = 93; /* TCPFastOpenCookieReqd */
pub const LINUX_MIB_TCPFASTOPENBLACKHOLE: i32 = 94; /* TCPFastOpenBlackholeDetect */
pub const LINUX_MIB_TCPSPURIOUS_RTX_HOSTQUEUES: i32 = 95; /* TCPSpuriousRtxHostQueues */
pub const LINUX_MIB_BUSYPOLLRXPACKETS: i32 = 96; /* BusyPollRxPackets */
pub const LINUX_MIB_TCPAUTOCORKING: i32 = 97; /* TCPAutoCorking */
pub const LINUX_MIB_TCPFROMZEROWINDOWADV: i32 = 98; /* TCPFromZeroWindowAdv */
pub const LINUX_MIB_TCPTOZEROWINDOWADV: i32 = 99; /* TCPToZeroWindowAdv */
pub const LINUX_MIB_TCPWANTZEROWINDOWADV: i32 = 100; /* TCPWantZeroWindowAdv */
pub const LINUX_MIB_TCPSYNRETRANS: i32 = 101; /* TCPSynRetrans */
pub const LINUX_MIB_TCPORIGDATASENT: i32 = 102; /* TCPOrigDataSent */
pub const LINUX_MIB_TCPHYSTARTTRAINDETECT: i32 = 103; /* TCPHystartTrainDetect */
pub const LINUX_MIB_TCPHYSTARTTRAINCWND: i32 = 104; /* TCPHystartTrainCwnd */
pub const LINUX_MIB_TCPHYSTARTDELAYDETECT: i32 = 105; /* TCPHystartDelayDetect */
pub const LINUX_MIB_TCPHYSTARTDELAYCWND: i32 = 106; /* TCPHystartDelayCwnd */
pub const LINUX_MIB_TCPACKSKIPPEDSYNRECV: i32 = 107; /* TCPACKSkippedSynRecv */
pub const LINUX_MIB_TCPACKSKIPPEDPAWS: i32 = 108; /* TCPACKSkippedPAWS */
pub const LINUX_MIB_TCPACKSKIPPEDSEQ: i32 = 109; /* TCPACKSkippedSeq */
pub const LINUX_MIB_TCPACKSKIPPEDFINWAIT2: i32 = 110; /* TCPACKSkippedFinWait2 */
pub const LINUX_MIB_TCPACKSKIPPEDTIMEWAIT: i32 = 111; /* TCPACKSkippedTimeWait */
pub const LINUX_MIB_TCPACKSKIPPEDCHALLENGE: i32 = 112; /* TCPACKSkippedChallenge */
pub const LINUX_MIB_TCPWINPROBE: i32 = 113; /* TCPWinProbe */
pub const LINUX_MIB_TCPKEEPALIVE: i32 = 114; /* TCPKeepAlive */
pub const LINUX_MIB_TCPMTUPFAIL: i32 = 115; /* TCPMTUPFail */
pub const LINUX_MIB_TCPMTUPSUCCESS: i32 = 116; /* TCPMTUPSuccess */
pub const LINUX_MIB_TCPDELIVERED: i32 = 117; /* TCPDelivered */
pub const LINUX_MIB_TCPDELIVEREDCE: i32 = 118; /* TCPDeliveredCE */
pub const LINUX_MIB_TCPACKCOMPRESSED: i32 = 119; /* TCPAckCompressed */
pub const LINUX_MIB_TCPZEROWINDOWDROP: i32 = 120; /* TCPZeroWindowDrop */
pub const LINUX_MIB_TCPRCVQDROP: i32 = 121; /* TCPRcvQDrop */
pub const LINUX_MIB_TCPWQUEUETOOBIG: i32 = 122; /* TCPWqueueTooBig */
pub const LINUX_MIB_TCPFASTOPENPASSIVEALTKEY: i32 = 123; /* TCPFastOpenPassiveAltKey */
pub const LINUX_MIB_TCPTIMEOUTREHASH: i32 = 124; /* TCPTimeoutRehash */
pub const LINUX_MIB_TCPDUPLICATEDATAREHASH: i32 = 125; /* TCPDuplicateDataRehash */
pub const LINUX_MIB_TCPDSACKRECVSEGS: i32 = 126; /* TCPDSACKRecvSegs */
pub const LINUX_MIB_TCPDSACKIGNOREDDUBIOUS: i32 = 127; /* TCPDSACKIgnoredDubious */
pub const LINUX_MIB_TCPMIGRATEREQSUCCESS: i32 = 128; /* TCPMigrateReqSuccess */
pub const LINUX_MIB_TCPMIGRATEREQFAILURE: i32 = 129; /* TCPMigrateReqFailure */
pub const LINUX_MIB_TCPPLBREHASH: i32 = 130; /* TCPPLBRehash */
pub const LINUX_MIB_TCPAOREQUIRED: i32 = 131; /* TCPAORequired */
pub const LINUX_MIB_TCPAOBAD: i32 = 132; /* TCPAOBad */
pub const LINUX_MIB_TCPAOKEYNOTFOUND: i32 = 133; /* TCPAOKeyNotFound */
pub const LINUX_MIB_TCPAOGOOD: i32 = 134; /* TCPAOGood */
pub const LINUX_MIB_TCPAODROPPEDICMPS: i32 = 135; /* TCPAODroppedIcmps */
pub const __LINUX_MIB_MAX: i32 = 136;


/* linux Xfrm mib definitions */
// C anonymous enum translated as i32 constants
pub const LINUX_MIB_XFRMNUM: i32 = 0;
pub const LINUX_MIB_XFRMINERROR: i32 = 1; /* XfrmInError */
pub const LINUX_MIB_XFRMINBUFFERERROR: i32 = 2; /* XfrmInBufferError */
pub const LINUX_MIB_XFRMINHDRERROR: i32 = 3; /* XfrmInHdrError */
pub const LINUX_MIB_XFRMINNOSTATES: i32 = 4; /* XfrmInNoStates */
pub const LINUX_MIB_XFRMINSTATEPROTOERROR: i32 = 5; /* XfrmInStateProtoError */
pub const LINUX_MIB_XFRMINSTATEMODEERROR: i32 = 6; /* XfrmInStateModeError */
pub const LINUX_MIB_XFRMINSTATESEQERROR: i32 = 7; /* XfrmInStateSeqError */
pub const LINUX_MIB_XFRMINSTATEEXPIRED: i32 = 8; /* XfrmInStateExpired */
pub const LINUX_MIB_XFRMINSTATEMISMATCH: i32 = 9; /* XfrmInStateMismatch */
pub const LINUX_MIB_XFRMINSTATEINVALID: i32 = 10; /* XfrmInStateInvalid */
pub const LINUX_MIB_XFRMINTMPLMISMATCH: i32 = 11; /* XfrmInTmplMismatch */
pub const LINUX_MIB_XFRMINNOPOLS: i32 = 12; /* XfrmInNoPols */
pub const LINUX_MIB_XFRMINPOLBLOCK: i32 = 13; /* XfrmInPolBlock */
pub const LINUX_MIB_XFRMINPOLERROR: i32 = 14; /* XfrmInPolError */
pub const LINUX_MIB_XFRMOUTERROR: i32 = 15; /* XfrmOutError */
pub const LINUX_MIB_XFRMOUTBUNDLEGENERROR: i32 = 16; /* XfrmOutBundleGenError */
pub const LINUX_MIB_XFRMOUTBUNDLECHECKERROR: i32 = 17; /* XfrmOutBundleCheckError */
pub const LINUX_MIB_XFRMOUTNOSTATES: i32 = 18; /* XfrmOutNoStates */
pub const LINUX_MIB_XFRMOUTSTATEPROTOERROR: i32 = 19; /* XfrmOutStateProtoError */
pub const LINUX_MIB_XFRMOUTSTATEMODEERROR: i32 = 20; /* XfrmOutStateModeError */
pub const LINUX_MIB_XFRMOUTSTATESEQERROR: i32 = 21; /* XfrmOutStateSeqError */
pub const LINUX_MIB_XFRMOUTSTATEEXPIRED: i32 = 22; /* XfrmOutStateExpired */
pub const LINUX_MIB_XFRMOUTPOLBLOCK: i32 = 23; /* XfrmOutPolBlock */
pub const LINUX_MIB_XFRMOUTPOLDEAD: i32 = 24; /* XfrmOutPolDead */
pub const LINUX_MIB_XFRMOUTPOLERROR: i32 = 25; /* XfrmOutPolError */
pub const LINUX_MIB_XFRMFWDHDRERROR: i32 = 26; /* XfrmFwdHdrError*/
pub const LINUX_MIB_XFRMOUTSTATEINVALID: i32 = 27; /* XfrmOutStateInvalid */
pub const LINUX_MIB_XFRMACQUIREERROR: i32 = 28; /* XfrmAcquireError */
pub const LINUX_MIB_XFRMOUTSTATEDIRERROR: i32 = 29; /* XfrmOutStateDirError */
pub const LINUX_MIB_XFRMINSTATEDIRERROR: i32 = 30; /* XfrmInStateDirError */
pub const LINUX_MIB_XFRMINIPTFSERROR: i32 = 31; /* XfrmInIptfsError */
pub const LINUX_MIB_XFRMOUTNOQSPACE: i32 = 32; /* XfrmOutNoQueueSpace */
pub const __LINUX_MIB_XFRMMAX: i32 = 33;


/* linux TLS mib definitions */
// C anonymous enum translated as i32 constants
pub const LINUX_MIB_TLSNUM: i32 = 0;
pub const LINUX_MIB_TLSCURRTXSW: i32 = 1; /* TlsCurrTxSw */
pub const LINUX_MIB_TLSCURRRXSW: i32 = 2; /* TlsCurrRxSw */
pub const LINUX_MIB_TLSCURRTXDEVICE: i32 = 3; /* TlsCurrTxDevice */
pub const LINUX_MIB_TLSCURRRXDEVICE: i32 = 4; /* TlsCurrRxDevice */
pub const LINUX_MIB_TLSTXSW: i32 = 5; /* TlsTxSw */
pub const LINUX_MIB_TLSRXSW: i32 = 6; /* TlsRxSw */
pub const LINUX_MIB_TLSTXDEVICE: i32 = 7; /* TlsTxDevice */
pub const LINUX_MIB_TLSRXDEVICE: i32 = 8; /* TlsRxDevice */
pub const LINUX_MIB_TLSDECRYPTERROR: i32 = 9; /* TlsDecryptError */
pub const LINUX_MIB_TLSRXDEVICERESYNC: i32 = 10; /* TlsRxDeviceResync */
pub const LINUX_MIB_TLSDECRYPTRETRY: i32 = 11; /* TlsDecryptRetry */
pub const LINUX_MIB_TLSRXNOPADVIOL: i32 = 12; /* TlsRxNoPadViolation */
pub const LINUX_MIB_TLSRXREKEYOK: i32 = 13; /* TlsRxRekeyOk */
pub const LINUX_MIB_TLSRXREKEYERROR: i32 = 14; /* TlsRxRekeyError */
pub const LINUX_MIB_TLSTXREKEYOK: i32 = 15; /* TlsTxRekeyOk */
pub const LINUX_MIB_TLSTXREKEYERROR: i32 = 16; /* TlsTxRekeyError */
pub const LINUX_MIB_TLSRXREKEYRECEIVED: i32 = 17; /* TlsRxRekeyReceived */
pub const __LINUX_MIB_TLSMAX: i32 = 18;

