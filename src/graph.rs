#[macro_export]
macro_rules! visitor_trait {
    ($trait_name:ident { $($name:ident ( $($(#[$leaf:ident])? $child:ident : $ty:ty),+ )),+ }) => {
        paste::paste! {
            pub trait [<Mut $trait_name>] {
                $(visitor_trait!(@node $name ( $($(#[$leaf])? $child : $ty),+ ));)+
            }
        }
    };
    (@node $name:ident ( $($(#[$leaf:ident])? $child:ident : $ty:ty),+ )) => {
        paste::paste! {
            fn [<visit_ $name>](&mut self, $($child: &mut $ty),+) {
                $(visitor_trait!(@accept $($leaf)? self $child);)+
            }
        }
    };

    (@accept $self:ident $child:ident) => {
        MutVisitable::accept($child, $self)
    };

    (@accept leaf $self:ident $child:ident) => {
    };
}
