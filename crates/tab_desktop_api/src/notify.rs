use std::hash::Hash;

use dashmap::DashMap;
use fnv::FnvBuildHasher;
use tab_proto::fig::NotificationType;

pub struct NotificationHandler<K>
where
    K: Hash + Eq + PartialEq,
{
    pub subscriptions: DashMap<(K, NotificationType), i64, FnvBuildHasher>,
}
