//! linux-parity: complete
//! linux-source: vendor/linux/net/sunrpc/xprtrdma/module.c
//! test-origin: linux:vendor/linux/net/sunrpc/xprtrdma/module.c
//! RPC/RDMA module init and cleanup ordering.

extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleMetadata {
    pub author: &'static str,
    pub description: &'static str,
    pub license: &'static str,
    pub aliases: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RpcRdmaState {
    pub ib_client_registered: bool,
    pub svc_rdma_initialized: bool,
    pub xprt_rdma_initialized: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RpcRdmaInitPlan {
    pub ib_client_register_rc: i32,
    pub svc_rdma_init_rc: i32,
    pub xprt_rdma_init_rc: i32,
}

impl RpcRdmaInitPlan {
    pub const fn success() -> Self {
        Self {
            ib_client_register_rc: 0,
            svc_rdma_init_rc: 0,
            xprt_rdma_init_rc: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RpcRdmaStep {
    IbClientRegister,
    SvcRdmaInit,
    XprtRdmaInit,
    XprtRdmaCleanup,
    SvcRdmaCleanup,
    IbClientUnregister,
}

pub const RPC_RDMA_METADATA: ModuleMetadata = ModuleMetadata {
    author: "Open Grid Computing and Network Appliance, Inc.",
    description: "RPC/RDMA Transport",
    license: "Dual BSD/GPL",
    aliases: &["svcrdma", "xprtrdma", "rpcrdma6"],
};

pub fn rpc_rdma_init(
    state: &mut RpcRdmaState,
    plan: RpcRdmaInitPlan,
    log: &mut Vec<RpcRdmaStep>,
) -> Result<(), i32> {
    log.push(RpcRdmaStep::IbClientRegister);
    if plan.ib_client_register_rc != 0 {
        return Err(plan.ib_client_register_rc);
    }
    state.ib_client_registered = true;

    log.push(RpcRdmaStep::SvcRdmaInit);
    if plan.svc_rdma_init_rc != 0 {
        state.ib_client_registered = false;
        log.push(RpcRdmaStep::IbClientUnregister);
        return Err(plan.svc_rdma_init_rc);
    }
    state.svc_rdma_initialized = true;

    log.push(RpcRdmaStep::XprtRdmaInit);
    if plan.xprt_rdma_init_rc != 0 {
        state.svc_rdma_initialized = false;
        log.push(RpcRdmaStep::SvcRdmaCleanup);
        state.ib_client_registered = false;
        log.push(RpcRdmaStep::IbClientUnregister);
        return Err(plan.xprt_rdma_init_rc);
    }
    state.xprt_rdma_initialized = true;

    Ok(())
}

pub fn rpc_rdma_cleanup(state: &mut RpcRdmaState, log: &mut Vec<RpcRdmaStep>) {
    if state.xprt_rdma_initialized {
        state.xprt_rdma_initialized = false;
        log.push(RpcRdmaStep::XprtRdmaCleanup);
    }
    if state.svc_rdma_initialized {
        state.svc_rdma_initialized = false;
        log.push(RpcRdmaStep::SvcRdmaCleanup);
    }
    if state.ib_client_registered {
        state.ib_client_registered = false;
        log.push(RpcRdmaStep::IbClientUnregister);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_rdma_module_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/sunrpc/xprtrdma/module.c"
        ));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("#include <trace/events/rpcrdma.h>"));
        assert!(
            source.contains("MODULE_AUTHOR(\"Open Grid Computing and Network Appliance, Inc.\");")
        );
        assert!(source.contains("MODULE_DESCRIPTION(\"RPC/RDMA Transport\");"));
        assert!(source.contains("MODULE_LICENSE(\"Dual BSD/GPL\");"));
        assert!(source.contains("MODULE_ALIAS(\"svcrdma\");"));
        assert!(source.contains("MODULE_ALIAS(\"xprtrdma\");"));
        assert!(source.contains("MODULE_ALIAS(\"rpcrdma6\");"));
        assert!(source.contains("static void __exit rpc_rdma_cleanup(void)"));
        assert!(source.contains("xprt_rdma_cleanup();"));
        assert!(source.contains("svc_rdma_cleanup();"));
        assert!(source.contains("rpcrdma_ib_client_unregister();"));
        assert!(source.contains("rc = rpcrdma_ib_client_register();"));
        assert!(source.contains("rc = svc_rdma_init();"));
        assert!(source.contains("rc = xprt_rdma_init();"));
        assert!(source.contains("out_svc_rdma:"));
        assert!(source.contains("out_ib_client:"));
        assert!(source.contains("module_init(rpc_rdma_init);"));
        assert!(source.contains("module_exit(rpc_rdma_cleanup);"));

        assert_eq!(RPC_RDMA_METADATA.description, "RPC/RDMA Transport");
        assert_eq!(
            RPC_RDMA_METADATA.aliases,
            ["svcrdma", "xprtrdma", "rpcrdma6"]
        );
    }

    #[test]
    fn rpc_rdma_init_rolls_back_in_linux_label_order() {
        let mut state = RpcRdmaState::default();
        let mut log = Vec::new();
        assert_eq!(
            rpc_rdma_init(
                &mut state,
                RpcRdmaInitPlan {
                    ib_client_register_rc: 0,
                    svc_rdma_init_rc: 0,
                    xprt_rdma_init_rc: -5,
                },
                &mut log,
            ),
            Err(-5)
        );
        assert_eq!(
            log,
            alloc::vec![
                RpcRdmaStep::IbClientRegister,
                RpcRdmaStep::SvcRdmaInit,
                RpcRdmaStep::XprtRdmaInit,
                RpcRdmaStep::SvcRdmaCleanup,
                RpcRdmaStep::IbClientUnregister,
            ]
        );
        assert_eq!(state, RpcRdmaState::default());

        log.clear();
        assert_eq!(
            rpc_rdma_init(&mut state, RpcRdmaInitPlan::success(), &mut log),
            Ok(())
        );
        assert!(state.ib_client_registered);
        assert!(state.svc_rdma_initialized);
        assert!(state.xprt_rdma_initialized);

        log.clear();
        rpc_rdma_cleanup(&mut state, &mut log);
        assert_eq!(
            log,
            alloc::vec![
                RpcRdmaStep::XprtRdmaCleanup,
                RpcRdmaStep::SvcRdmaCleanup,
                RpcRdmaStep::IbClientUnregister,
            ]
        );
        assert_eq!(state, RpcRdmaState::default());
    }
}
