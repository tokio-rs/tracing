macro_rules! cfg_feature {
    ($name:literal, { $($item:item)* }) => {
        $(
            #[cfg(feature = $name)]
            #[cfg_attr(docsrs, doc(cfg(feature = $name)))]
            $item
        )*
    }
}
