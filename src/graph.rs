#[macro_export]
macro_rules! visitor_trait {
    ($trait_name:ident { $($name:ident ( $($(#[$node_ty:ident])? $child:ident : $ty:ty),+ )),+ }) => {
        paste::paste! {
            pub trait [<Mut $trait_name>] {
                $(visitor_trait!(@node $name ( $($(#[$node_ty])? $child : $ty),+ ));)+
            }
        }
    };
    (@node $name:ident ( $($(#[$node_ty:ident])? $child:ident : $ty:ty),+ )) => {
        paste::paste! {
            #[allow(unused)]
            fn [<visit_ $name>](&mut self, $($child: &mut $ty),+) {
                $(visitor_trait!(@visit $($node_ty)? self $child);)+
            }
        }
    };

    (@visit branch $self:ident $child:ident) => {
        $child.accept($self);
    };

    (@visit leaf $self:ident $child:ident) => {
    };

    (@visit $self:ident $child:ident) => {
        $child.walk($self)
    };
}
