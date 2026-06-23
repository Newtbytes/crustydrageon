#[macro_export]
macro_rules! visitor_trait {
    // TODO: implement marking parameters as leaf nodes
    ($trait_name:ident { $($name:ident $([$leaf:ident])? ( $($child:ident : $ty:ty),+ )),+ }) => {
        paste::paste! {
            pub trait [<Mut $trait_name>] {
                $(visitor_trait!(@node $($leaf)? $name ( $($child : $ty),+ ));)+
            }
        }
    };

    (@node $name:ident ( $($child:ident : $ty:ty),+ )) => {
        visitor_trait!(@branch $name ( $($child : $ty),+ ));
    };

    (@node leaf $name:ident ( $($child:ident : $ty:ty),+ )) => {
        visitor_trait!(@leaf $name ( $($child : $ty),+ ));
    };

    (@branch $name:ident ( $($child:ident : $ty:ty),+ )) => {
        paste::paste! {
            fn [<visit_ $name>](&mut self, $($child: &mut $ty),+) {
                $(MutVisitable::accept($child, self);)+
            }
        }
    };

    (@leaf $name:ident ( $($child:ident : $ty:ty),+ )) => {
        paste::paste! {
            fn [<visit_ $name>](&mut self, $($child: &mut $ty),+) {}
        }
    };
}
