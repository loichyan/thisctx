#[macro_export]
macro_rules! thisctx {
    (
        $(#[$enum_attr:meta])*
        $ty_vis:vis enum $name:ident {
            $($enum_body:tt)*
        }
    ) => {
        thisctx!(@def
            enum [
                head [ $(#[$enum_attr]) *$ty_vis $name ]
                body []
                queue [ $($enum_body)* ]
            ]
        );
    };

    // # start definition
    (@def
        enum [ $($enum:tt)* ]
    ) => {
        thisctx!(@def
            enum [ $($enum)* ]
            context [ ]
        );
    };
    // # unit variant
    (@def
        enum [
            head [ $($head:tt)* ]
            body [ $($body:tt)* ]
            queue [ $(#[$v_attr:meta])* $variant:ident, $($tail:tt)* ]
        ]
        context [ $($context:tt)* ]
    ) => {
        thisctx!(@def
            enum [
                head [ $($head)* ]
                body [ $(#[$v_attr])* $variant, $($body)* ]
                queue [ $($tail)* ]
            ]
            context [ head [ $variant ] body [ ; ] $($context)* ]
        );
    };
    // # struct variant
    (@def
        enum [
            head [ $($head:tt)* ]
            body [ $($body:tt)* ]
            queue [ $(#[$v_attr:meta])* $variant:ident { $($v_body:tt)* }, $($tail:tt)* ]
        ]
        context [ $($context:tt)* ]
    ) => {
        thisctx!(@def_enum_var
            enum [
                head [ $($head)* ]
                body [ $($body)* ]
                queue [ $($tail)* ]
            ]
            variant [
                head [ $(#[$v_attr])* $variant ]
                body [ ]
                queue [ $($v_body)* ]
            ]
            context [ head [ $variant ] body [ ; ] $($context)* ]
        );
    };
    // ## variant body
    // ### source field
    (@def_enum_var
        enum [
            head [ $($head:tt)* ]
            body [ $($body:tt)* ]
            queue [ $($queue:tt)* ]
        ]
        variant [
            head [ $(#[$v_attr:meta])* $variant:ident ]
            body [ $($v_body:tt)* ]
            queue [ @source $src:ident: $src_ty:ty, $($v_queue:tt)* ]
        ]
        context [ $($context:tt)* ]
    ) => {
        thisctx!(@def_enum_var
            enum [
                head [ $($head)* ]
                body [ $($body)* ]
                queue [ $($queue)* ]
            ]
            variant [
                head [ $(#[$v_attr]) *$variant ]
                body [ $src: $src_ty, $($v_body)* ]
                queue [ $($v_queue)* ]
            ]
            context [ $($context)* ]
        );
    };
    // ### context field
    (@def_enum_var
        enum [
            head [ $($head:tt)* ]
            body [ $($body:tt)* ]
            queue [ $($queue:tt)* ]
        ]
        variant [
            head [ $(#[$v_attr:meta])* $variant:ident ]
            body [ $($v_body:tt)* ]
            queue [ @context $ctx:ident: $(#[$ctx_attr:meta])* struct $ctx_body:tt, $($v_queue:tt)* ]
        ]
        context [ head [ $ctx_name:ident ] body [ ; ] $($context:tt)* ]
    ) => {
        thisctx!(@def_enum_var
            enum [
                head [ $($head)* ]
                body [ $($body)* ]
                queue [ $($queue)* ]
            ]
            variant [
                head [ $(#[$v_attr]) *$variant ]
                body [ $ctx: $variant, $($v_body)* ]
                queue [ $($v_queue)* ]
            ]
            context [ head [ $(#[$ctx_attr])* $ctx_name ] body [ $ctx_body ] $($context)* ]
        );
    };
    // ### finish enum variant definition
    (@def_enum_var
        enum [
            head [ $($head:tt)* ]
            body [ $($body:tt)* ]
            queue [ $($queue:tt)* ]
        ]
        variant [
            head [ $($v_head:tt)* ]
            body [ $($v_body:tt)* ]
            queue [ ]
        ]
        context [ $($context:tt)* ]
    ) => {
        thisctx!(@def
            enum [
                head [ $($head)* ]
                body [ $($v_head)* { $($v_body)* }, $($body)* ]
                queue [ $($queue)* ]
            ]
            context [ $($context)* ]
        );
    };
    // # finish definition
    (@def
        enum [
            head [ $(#[$enum_attr:meta]) *$ty_vis:vis $name:ident ]
            body [ $($body:tt)* ]
            queue [ ]
        ]
        context [ $(head [ $(#[$ctx_attr:meta])* $ctx:ident ] body [ $($ctx_body:tt)* ])* ]
    ) => {
        $(#[$enum_attr])*
        $ty_vis enum $name {
            $($body)*
        }
        $(
            $(#[$ctx_attr])*
            $ty_vis struct $ctx $($ctx_body)*
        )*
    };
}

#[cfg(test)]
mod test {
    #![allow(unused)]

    thisctx! {
        #[derive(Debug, thiserror::Error)]
        pub enum Error {
            #[error("I/O '{}': {src}", .ctx.path.display())]
            Io {
                @source
                src: std::io::Error,
                @context
                ctx:
                    #[derive(Debug)]
                    struct {
                        path: std::path::PathBuf,
                    },
            },
            #[error("Unit")]
            Unit,
        }
    }
}
