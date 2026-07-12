// SPDX-License-Identifier: GPL-2.0
/*
 * Compile-time ABI contract between the configured vendor modules and the
 * Rust objects exposed by Lupos.  This is built by Kbuild after olddefconfig,
 * so any vendor/config layout drift stops artifact production.
 */

#include <linux/fb.h>
#include <linux/bio.h>
#include <linux/blk-mq.h>
#include <linux/blkdev.h>
#include <linux/fs.h>
#include <linux/hrtimer.h>
#include <linux/kthread.h>
#include <linux/mount.h>
#include <linux/module.h>
#include <linux/netdevice.h>
#include <linux/pagemap.h>
#include <linux/pci.h>
#include <linux/scatterlist.h>
#include <linux/skbuff.h>
#include <linux/virtio.h>
#include <linux/workqueue.h>
#include <net/netdev_rx_queue.h>
#include <net/page_pool/types.h>

#define ABI_OFFSET(type, member, expected) \
	_Static_assert(offsetof(type, member) == (expected), \
		       #type "." #member " offset changed")
#define ABI_SIZE(type, expected) \
	_Static_assert(sizeof(type) == (expected), #type " size changed")

ABI_OFFSET(struct device, parent, 64);
ABI_OFFSET(struct device, p, 72);
ABI_OFFSET(struct device, init_name, 80);
ABI_OFFSET(struct device, type, 88);
ABI_OFFSET(struct device, bus, 96);
ABI_OFFSET(struct device, driver, 104);
ABI_OFFSET(struct device, platform_data, 112);
ABI_OFFSET(struct device, driver_data, 120);
ABI_OFFSET(struct device, dma_mask, 584);
ABI_OFFSET(struct device, coherent_dma_mask, 592);
ABI_OFFSET(struct device, dma_parms, 616);
ABI_SIZE(struct device, 760);

ABI_OFFSET(struct device_driver, name, 0);
ABI_OFFSET(struct device_driver, bus, 8);
ABI_OFFSET(struct device_driver, probe, 56);
ABI_OFFSET(struct device_driver, remove, 72);
ABI_OFFSET(struct device_driver, pm, 120);
ABI_OFFSET(struct device_driver, p, 136);
ABI_SIZE(struct device_driver, 152);

ABI_OFFSET(struct pci_dev, bus, 16);
ABI_OFFSET(struct pci_dev, devfn, 56);
ABI_OFFSET(struct pci_dev, driver, 120);
ABI_OFFSET(struct pci_dev, dma_mask, 128);
ABI_OFFSET(struct pci_dev, msi_addr_mask, 136);
ABI_OFFSET(struct pci_dev, error_state, 196);
ABI_OFFSET(struct pci_dev, dev, 200);
ABI_OFFSET(struct pci_dev, cfg_size, 960);
ABI_OFFSET(struct pci_dev, irq, 964);
ABI_OFFSET(struct pci_dev, resource, 968);
ABI_OFFSET(struct pci_dev, driver_exclusive_resource, 1672);
_Static_assert(DEVICE_COUNT_RESOURCE == 11,
	       "PCI resource count changed (CONFIG_PCI_IOV drift?)");
ABI_SIZE(struct pci_dev, 1944);

ABI_OFFSET(struct pci_driver, id_table, 8);
ABI_OFFSET(struct pci_driver, probe, 16);
ABI_OFFSET(struct pci_driver, groups, 88);
ABI_OFFSET(struct pci_driver, driver, 104);
ABI_SIZE(struct pci_driver, 288);

ABI_OFFSET(struct pci_bus, parent, 16);
ABI_OFFSET(struct pci_bus, children, 24);
ABI_OFFSET(struct pci_bus, devices, 40);
ABI_OFFSET(struct pci_bus, self, 56);
ABI_OFFSET(struct pci_bus, resource, 80);
ABI_OFFSET(struct pci_bus, resources, 112);
ABI_OFFSET(struct pci_bus, busn_res, 128);
ABI_OFFSET(struct pci_bus, ops, 192);
ABI_OFFSET(struct pci_bus, sysdata, 200);
ABI_OFFSET(struct pci_bus, number, 216);
ABI_OFFSET(struct pci_bus, primary, 217);
ABI_OFFSET(struct pci_bus, name, 220);
ABI_OFFSET(struct pci_bus, bridge_ctl, 268);
ABI_OFFSET(struct pci_bus, bus_flags, 270);
ABI_OFFSET(struct pci_bus, bridge, 272);
ABI_OFFSET(struct pci_bus, dev, 280);
ABI_SIZE(struct pci_bus, 1048);

ABI_OFFSET(struct virtio_device, config_lock, 8);
ABI_OFFSET(struct virtio_device, vqs_list_lock, 12);
ABI_OFFSET(struct virtio_device, dev, 16);
ABI_OFFSET(struct virtio_device, id, 776);
ABI_OFFSET(struct virtio_device, config, 784);
ABI_OFFSET(struct virtio_device, vringh_config, 792);
ABI_OFFSET(struct virtio_device, map, 800);
ABI_OFFSET(struct virtio_device, vqs, 808);
ABI_OFFSET(struct virtio_device, features, 824);
ABI_OFFSET(struct virtio_device, priv, 840);
ABI_OFFSET(struct virtio_device, vmap, 848);
ABI_SIZE(struct virtio_device, 856);

ABI_OFFSET(struct virtio_driver, driver, 0);
ABI_OFFSET(struct virtio_driver, id_table, 152);
ABI_OFFSET(struct virtio_driver, feature_table, 160);
ABI_OFFSET(struct virtio_driver, validate, 192);
ABI_OFFSET(struct virtio_driver, probe, 200);
ABI_OFFSET(struct virtio_driver, shutdown, 264);
ABI_SIZE(struct virtio_driver, 272);

ABI_OFFSET(struct file, f_mapping, 16);
ABI_OFFSET(struct file, f_inode, 32);
ABI_SIZE(struct file, 176);
ABI_OFFSET(struct inode, i_mapping, 48);
ABI_SIZE(struct inode, 544);
ABI_SIZE(struct address_space, 152);

ABI_OFFSET(struct fb_info, node, 4);
ABI_OFFSET(struct fb_info, var, 64);
ABI_OFFSET(struct fb_info, fix, 224);
ABI_OFFSET(struct fb_info, cmap, 592);
ABI_OFFSET(struct fb_info, screen_base, 688);
ABI_OFFSET(struct fb_info, par, 728);
ABI_SIZE(struct fb_info, 744);

ABI_OFFSET(struct delayed_work, timer, 32);
ABI_OFFSET(struct kthread_work, func, 16);
ABI_OFFSET(struct vfsmount, mnt_sb, 8);
ABI_OFFSET(struct hrtimer, node.expires, 40);
ABI_OFFSET(struct hrtimer, _softexpires, 64);
ABI_OFFSET(struct hrtimer, function, 72);
ABI_SIZE(struct hrtimer, 80);

/* Block-core layouts are configuration-sensitive.  In particular SMP,
 * BLK_RQ_ALLOC_TIME, PM, BLK_CGROUP, MUTEX_SPIN_ON_OWNER and BLK_DEBUG_FS
 * all change records dereferenced by the vendor storage drivers. */
ABI_SIZE(spinlock_t, 4);
ABI_SIZE(struct mutex, 24);
ABI_SIZE(struct queue_limits, 192);
ABI_OFFSET(struct queue_limits, logical_block_size, 56);

ABI_OFFSET(struct blk_mq_ops, poll, 64);
ABI_OFFSET(struct blk_mq_ops, complete, 72);
ABI_OFFSET(struct blk_mq_ops, map_queues, 128);
ABI_OFFSET(struct blk_mq_ops, show_rq, 136);
ABI_SIZE(struct blk_mq_ops, 144);

ABI_OFFSET(struct blk_mq_tag_set, driver_data, 88);
ABI_OFFSET(struct blk_mq_tag_set, tags, 96);
ABI_OFFSET(struct blk_mq_tag_set, shared_tags, 104);
ABI_OFFSET(struct blk_mq_tag_set, tag_list_lock, 112);
ABI_OFFSET(struct blk_mq_tag_set, tag_list, 136);
ABI_OFFSET(struct blk_mq_tag_set, srcu, 152);
ABI_OFFSET(struct blk_mq_tag_set, tags_srcu, 160);
ABI_OFFSET(struct blk_mq_tag_set, update_nr_hwq_lock, 192);
ABI_SIZE(struct blk_mq_tag_set, 224);

_Static_assert(_Alignof(struct blk_mq_hw_ctx) == 64,
	       "struct blk_mq_hw_ctx alignment changed");
ABI_OFFSET(struct blk_mq_hw_ctx, queue, 184);
ABI_OFFSET(struct blk_mq_hw_ctx, fq, 192);
ABI_OFFSET(struct blk_mq_hw_ctx, driver_data, 200);
ABI_OFFSET(struct blk_mq_hw_ctx, ctx_map, 208);
ABI_OFFSET(struct blk_mq_hw_ctx, dispatch_from, 240);
ABI_OFFSET(struct blk_mq_hw_ctx, dispatch_busy, 248);
ABI_OFFSET(struct blk_mq_hw_ctx, type, 252);
ABI_OFFSET(struct blk_mq_hw_ctx, nr_ctx, 254);
ABI_OFFSET(struct blk_mq_hw_ctx, ctxs, 256);
ABI_OFFSET(struct blk_mq_hw_ctx, tags, 320);
ABI_OFFSET(struct blk_mq_hw_ctx, sched_tags, 328);
ABI_OFFSET(struct blk_mq_hw_ctx, numa_node, 336);
ABI_OFFSET(struct blk_mq_hw_ctx, queue_num, 340);
ABI_OFFSET(struct blk_mq_hw_ctx, nr_active, 344);
ABI_SIZE(struct blk_mq_hw_ctx, 512);

ABI_OFFSET(struct request, q, 0);
ABI_OFFSET(struct request, mq_hctx, 16);
ABI_OFFSET(struct request, cmd_flags, 24);
ABI_OFFSET(struct request, rq_flags, 28);
ABI_OFFSET(struct request, tag, 32);
ABI_OFFSET(struct request, internal_tag, 36);
ABI_OFFSET(struct request, timeout, 40);
ABI_OFFSET(struct request, __data_len, 44);
ABI_OFFSET(struct request, __sector, 48);
ABI_OFFSET(struct request, bio, 56);
ABI_OFFSET(struct request, biotail, 64);
ABI_OFFSET(struct request, queuelist, 72);
ABI_OFFSET(struct request, part, 88);
ABI_OFFSET(struct request, alloc_time_ns, 96);
ABI_OFFSET(struct request, start_time_ns, 104);
ABI_OFFSET(struct request, io_start_time_ns, 112);
ABI_OFFSET(struct request, stats_sectors, 120);
ABI_OFFSET(struct request, nr_phys_segments, 122);
ABI_OFFSET(struct request, nr_integrity_segments, 124);
ABI_OFFSET(struct request, phys_gap_bit, 126);
ABI_OFFSET(struct request, state, 128);
ABI_OFFSET(struct request, ref, 132);
ABI_OFFSET(struct request, deadline, 136);
ABI_OFFSET(struct request, hash, 144);
ABI_OFFSET(struct request, special_vec, 160);
ABI_OFFSET(struct request, elv, 184);
ABI_OFFSET(struct request, flush, 208);
ABI_OFFSET(struct request, fifo_time, 224);
ABI_OFFSET(struct request, end_io, 232);
ABI_OFFSET(struct request, end_io_data, 240);
ABI_SIZE(struct request, 248);

ABI_OFFSET(struct request_queue, queue_hw_ctx, 56);
ABI_OFFSET(struct request_queue, disk, 96);
ABI_OFFSET(struct request_queue, mq_kobj, 104);
ABI_OFFSET(struct request_queue, limits, 112);
ABI_OFFSET(struct request_queue, pm_only, 316);
ABI_OFFSET(struct request_queue, stats, 320);
ABI_OFFSET(struct request_queue, rq_qos, 328);
ABI_OFFSET(struct request_queue, rq_qos_mutex, 336);
ABI_OFFSET(struct request_queue, id, 360);
ABI_OFFSET(struct request_queue, nr_requests, 364);
ABI_OFFSET(struct request_queue, async_depth, 368);
ABI_OFFSET(struct request_queue, timeout, 376);
ABI_OFFSET(struct request_queue, timeout_work, 416);
ABI_OFFSET(struct request_queue, nr_active_requests_shared_tags, 448);
ABI_OFFSET(struct request_queue, sched_shared_tags, 456);
ABI_OFFSET(struct request_queue, icq_list, 464);
ABI_OFFSET(struct request_queue, node, 536);
ABI_OFFSET(struct request_queue, requeue_lock, 540);
ABI_OFFSET(struct request_queue, requeue_list, 544);
ABI_OFFSET(struct request_queue, requeue_work, 560);
ABI_OFFSET(struct request_queue, fq, 648);
ABI_OFFSET(struct request_queue, flush_list, 656);
ABI_OFFSET(struct request_queue, elevator_lock, 672);
ABI_OFFSET(struct request_queue, sysfs_lock, 696);
ABI_OFFSET(struct request_queue, limits_lock, 720);
ABI_OFFSET(struct request_queue, unused_hctx_list, 744);
ABI_OFFSET(struct request_queue, unused_hctx_lock, 760);
ABI_OFFSET(struct request_queue, mq_freeze_depth, 764);
ABI_OFFSET(struct request_queue, rcu_head, 768);
ABI_OFFSET(struct request_queue, mq_freeze_wq, 784);
ABI_OFFSET(struct request_queue, mq_freeze_lock, 808);
ABI_OFFSET(struct request_queue, tag_set, 832);
ABI_OFFSET(struct request_queue, tag_set_list, 840);
ABI_OFFSET(struct request_queue, debugfs_dir, 856);
ABI_OFFSET(struct request_queue, sched_debugfs_dir, 864);
ABI_OFFSET(struct request_queue, rqos_debugfs_dir, 872);
ABI_OFFSET(struct request_queue, debugfs_mutex, 880);
ABI_SIZE(struct request_queue, 904);

ABI_OFFSET(struct bio, bi_iter, 40);
ABI_OFFSET(struct bio, bi_end_io, 64);
ABI_OFFSET(struct bio, bi_private, 72);
ABI_OFFSET(struct bio, bi_blkg, 80);
ABI_OFFSET(struct bio, issue_time_ns, 88);
ABI_OFFSET(struct bio, bi_iocost_cost, 96);
ABI_OFFSET(struct bio, bi_vcnt, 104);
ABI_OFFSET(struct bio, bi_max_vecs, 106);
ABI_OFFSET(struct bio, __bi_cnt, 108);
ABI_OFFSET(struct bio, bi_pool, 112);
ABI_SIZE(struct bio, 120);
ABI_OFFSET(struct scatterlist, dma_address, 16);
ABI_OFFSET(struct scatterlist, dma_length, 24);
ABI_OFFSET(struct scatterlist, dma_flags, 28);
ABI_SIZE(struct scatterlist, 32);

/* Network-driver hot-path records embedded in or dereferenced by
 * virtio_net.ko.  These layouts are especially sensitive to SMP, NETPOLL,
 * PAGE_POOL, XPS, RPS, BQL, SYSFS and skb feature configuration. */
_Static_assert(_Alignof(struct net_device) == 64,
	       "struct net_device alignment changed");
ABI_OFFSET(struct net_device, netdev_ops, 8);
ABI_OFFSET(struct net_device, _tx, 24);
ABI_OFFSET(struct net_device, real_num_tx_queues, 40);
ABI_OFFSET(struct net_device, num_tc, 54);
ABI_OFFSET(struct net_device, mtu, 56);
ABI_OFFSET(struct net_device, xps_maps, 128);
ABI_OFFSET(struct net_device, state, 168);
ABI_OFFSET(struct net_device, flags, 176);
ABI_OFFSET(struct net_device, ifindex, 224);
ABI_OFFSET(struct net_device, real_num_rx_queues, 228);
ABI_OFFSET(struct net_device, _rx, 232);
ABI_OFFSET(struct net_device, name, 288);
ABI_OFFSET(struct net_device, addr_len, 808);
ABI_OFFSET(struct net_device, priv_len, 824);
ABI_OFFSET(struct net_device, dev_addr, 1088);
ABI_OFFSET(struct net_device, num_rx_queues, 1096);
ABI_OFFSET(struct net_device, num_tx_queues, 1176);
ABI_OFFSET(struct net_device, tx_global_lock, 1196);
ABI_OFFSET(struct net_device, reg_state, 1432);
ABI_OFFSET(struct net_device, dev_addr_shadow, 2472);
ABI_OFFSET(struct net_device, napi_config, 2544);
ABI_OFFSET(struct net_device, num_napi_configs, 2552);
ABI_SIZE(struct net_device, 2624);

_Static_assert(_Alignof(struct netdev_queue) == 64,
	       "struct netdev_queue alignment changed");
ABI_OFFSET(struct netdev_queue, dev, 0);
ABI_OFFSET(struct netdev_queue, dql, 128);
ABI_OFFSET(struct netdev_queue, _xmit_lock, 256);
ABI_OFFSET(struct netdev_queue, xmit_lock_owner, 260);
ABI_OFFSET(struct netdev_queue, state, 272);
ABI_OFFSET(struct netdev_queue, napi, 280);
ABI_OFFSET(struct netdev_queue, numa_node, 288);
ABI_SIZE(struct netdev_queue, 320);
ABI_OFFSET(struct netdev_rx_queue, dev, 152);
ABI_OFFSET(struct netdev_rx_queue, napi, 160);
ABI_SIZE(struct netdev_rx_queue, 256);

ABI_OFFSET(struct napi_struct, state, 0);
ABI_OFFSET(struct napi_struct, poll_list, 8);
ABI_OFFSET(struct napi_struct, weight, 24);
ABI_OFFSET(struct napi_struct, poll, 32);
ABI_OFFSET(struct napi_struct, poll_owner, 40);
ABI_OFFSET(struct napi_struct, list_owner, 44);
ABI_OFFSET(struct napi_struct, dev, 48);
ABI_OFFSET(struct napi_struct, gro.rx_list, 264);
ABI_OFFSET(struct napi_struct, gro_flush_timeout, 376);
ABI_OFFSET(struct napi_struct, irq_suspend_timeout, 384);
ABI_OFFSET(struct napi_struct, defer_hard_irqs, 392);
ABI_OFFSET(struct napi_struct, napi_id, 396);
ABI_OFFSET(struct napi_struct, dev_list, 400);
ABI_OFFSET(struct napi_struct, napi_hash_node, 416);
ABI_OFFSET(struct napi_struct, irq, 432);
ABI_OFFSET(struct napi_struct, napi_rmap_idx, 496);
ABI_OFFSET(struct napi_struct, config, 504);
ABI_SIZE(struct napi_struct, 512);
ABI_OFFSET(struct napi_config, gro_flush_timeout, 0);
ABI_OFFSET(struct napi_config, irq_suspend_timeout, 8);
ABI_OFFSET(struct napi_config, defer_hard_irqs, 16);
ABI_OFFSET(struct napi_config, napi_id, 36);
ABI_SIZE(struct napi_config, 40);

ABI_OFFSET(struct sk_buff, dev, 16);
ABI_OFFSET(struct sk_buff, len, 112);
ABI_OFFSET(struct sk_buff, data_len, 116);
ABI_OFFSET(struct sk_buff, mac_len, 120);
_Static_assert(CLONED_OFFSET == 126, "struct sk_buff cloned flags moved");
_Static_assert(PKT_TYPE_OFFSET == 128, "struct sk_buff pkt_type flags moved");
ABI_OFFSET(struct sk_buff, alloc_cpu, 136);
ABI_OFFSET(struct sk_buff, napi_id, 160);
ABI_OFFSET(struct sk_buff, protocol, 180);
ABI_OFFSET(struct sk_buff, transport_header, 182);
ABI_OFFSET(struct sk_buff, network_header, 184);
ABI_OFFSET(struct sk_buff, mac_header, 186);
ABI_OFFSET(struct sk_buff, tail, 188);
ABI_OFFSET(struct sk_buff, end, 192);
ABI_OFFSET(struct sk_buff, head, 200);
ABI_OFFSET(struct sk_buff, data, 208);
ABI_OFFSET(struct sk_buff, truesize, 216);
ABI_OFFSET(struct sk_buff, users, 220);
ABI_SIZE(struct sk_buff, 232);
ABI_OFFSET(struct skb_shared_info, nr_frags, 2);
ABI_OFFSET(struct skb_shared_info, dataref, 32);
ABI_OFFSET(struct skb_shared_info, frags, 48);
ABI_SIZE(struct skb_shared_info, 320);

ABI_OFFSET(struct page_pool_params, slow, 48);
ABI_OFFSET(struct page_pool_params, slow.flags, 60);
ABI_OFFSET(struct page_pool_params, slow.init_callback, 64);
ABI_SIZE(struct page_pool_params, 80);
_Static_assert(_Alignof(struct page_pool) == 64,
	       "struct page_pool alignment changed");
ABI_OFFSET(struct page_pool, cpuid, 48);
ABI_OFFSET(struct page_pool, pages_state_hold_cnt, 52);
ABI_OFFSET(struct page_pool, frag_users, 64);
ABI_OFFSET(struct page_pool, frag_page, 72);
ABI_OFFSET(struct page_pool, frag_offset, 80);
ABI_OFFSET(struct page_pool, user_cnt, 1572);
ABI_OFFSET(struct page_pool, slow, 1584);
ABI_SIZE(struct page_pool, 1664);

ABI_OFFSET(struct page, page_type, 48);
ABI_OFFSET(struct page, _mapcount, 48);
ABI_OFFSET(struct page, _refcount, 52);
ABI_SIZE(struct page, 64);

ABI_OFFSET(struct block_device, bd_stats, 32);
ABI_OFFSET(struct block_device, bd_holder_lock, 96);
ABI_OFFSET(struct block_device, bd_holders, 120);
ABI_OFFSET(struct block_device, bd_device, 192);
ABI_SIZE(struct block_device, 952);

ABI_OFFSET(struct gendisk, bio_split, 96);
ABI_OFFSET(struct gendisk, flags, 344);
ABI_OFFSET(struct gendisk, state, 352);
ABI_SIZE(struct gendisk, 576);

/* Every selected .ko embeds this configured struct in
 * .gnu.linkonce.this_module.  The Rust loader validates the same size before
 * its lifecycle gate, so configuration drift must stop artifact production.
 */
ABI_OFFSET(struct module, name, 24);
_Static_assert(MODULE_NAME_LEN == 56, "module name capacity changed");
ABI_OFFSET(struct module, syms, 216);
ABI_OFFSET(struct module, flagstab, 232);
ABI_OFFSET(struct module, num_syms, 240);
ABI_OFFSET(struct module, kp, 272);
ABI_OFFSET(struct module, num_kp, 280);
ABI_OFFSET(struct module, init, 304);
ABI_OFFSET(struct module, num_bugs, 832);
ABI_OFFSET(struct module, bug_list, 840);
ABI_OFFSET(struct module, bug_table, 856);
ABI_OFFSET(struct module, source_list, 984);
ABI_OFFSET(struct module, target_list, 1000);
ABI_OFFSET(struct module, exit, 1016);
ABI_OFFSET(struct module, refcnt, 1024);
ABI_SIZE(struct module, 1088);
ABI_SIZE(struct kernel_param, 40);
ABI_SIZE(struct bug_entry, 16);

MODULE_LICENSE("GPL");
