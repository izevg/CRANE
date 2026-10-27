import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { cx, type WithClassName } from "~/shared/lib/core";

export function Menu(props: MenuPrimitive.Root.Props) {
  return <MenuPrimitive.Root {...props} />;
}

export function MenuTrigger(props: MenuPrimitive.Trigger.Props) {
  return <MenuPrimitive.Trigger data-slot="menu-trigger" {...props} />;
}

export function MenuContent({
  align = "start",
  side = "bottom",
  sideOffset = 4,
  className,
  ...props
}: WithClassName<MenuPrimitive.Popup.Props> &
  Pick<MenuPrimitive.Positioner.Props, "align" | "side" | "sideOffset">) {
  return (
    <MenuPrimitive.Portal>
      <MenuPrimitive.Positioner
        className="isolate z-50"
        align={align}
        side={side}
        sideOffset={sideOffset}
      >
        <MenuPrimitive.Popup
          data-slot="menu-content"
          className={cx(
            "anim-popup z-50 max-h-(--available-height) min-w-40 overflow-x-hidden overflow-y-auto rounded-md bg-popover p-1 text-popover-foreground shadow-card",
            className,
          )}
          {...props}
        />
      </MenuPrimitive.Positioner>
    </MenuPrimitive.Portal>
  );
}

export function MenuGroup(props: MenuPrimitive.Group.Props) {
  return <MenuPrimitive.Group data-slot="menu-group" {...props} />;
}

export function MenuLabel({ className, ...props }: WithClassName<MenuPrimitive.GroupLabel.Props>) {
  return (
    <MenuPrimitive.GroupLabel
      data-slot="menu-label"
      className={cx("px-2 py-1 text-caption text-muted-foreground", className)}
      {...props}
    />
  );
}

type MenuItemProps = WithClassName<MenuPrimitive.Item.Props> & {
  variant?: "default" | "destructive";
};

const ITEM_VARIANT = {
  default: "",
  destructive: "text-error data-highlighted:bg-error-bg",
} satisfies Record<NonNullable<MenuItemProps["variant"]>, string>;

export function MenuItem({ className, variant = "default", ...props }: MenuItemProps) {
  return (
    <MenuPrimitive.Item
      data-slot="menu-item"
      data-variant={variant}
      className={cx(
        "relative flex h-9 cursor-default select-none items-center gap-2 rounded-sm px-2 text-body data-highlighted:bg-accent data-disabled:cursor-not-allowed data-disabled:text-muted-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        ITEM_VARIANT[variant],
        className,
      )}
      {...props}
    />
  );
}

export function MenuSeparator({
  className,
  ...props
}: WithClassName<MenuPrimitive.Separator.Props>) {
  return (
    <MenuPrimitive.Separator
      data-slot="menu-separator"
      className={cx("-mx-1 my-1 h-px bg-border", className)}
      {...props}
    />
  );
}
