application_id = 'org.fareedsh.Moviola'

scalable_app_dir = 'hicolor' / 'scalable' / 'apps'
install_data(
  scalable_app_dir / ('@0@.svg').format(application_id),
  install_dir: iconsdir / scalable_app_dir
)

symbolic_app_dir = 'hicolor' / 'symbolic' / 'apps'
install_data(
  symbolic_app_dir / ('@0@-symbolic.svg').format(application_id),
  install_dir: iconsdir / symbolic_app_dir
)

action_dir = 'hicolor' / 'scalable' / 'actions'
action_icons = [
    action_dir / 'crop-symbolic.svg',
    action_dir / 'play-symbolic.svg',
    action_dir / 'pause-symbolic.svg',
    action_dir / 'rotate-left-symbolic.svg',
    action_dir / 'rotate-right-symbolic.svg',
    action_dir / 'panorama-horizontal-symbolic.svg',
    action_dir / 'panorama-vertical-symbolic.svg',
    action_dir / 'video-encoder-symbolic.svg',
]

install_data(
    action_icons,
    install_dir: pkgdatadir / 'icons/hicolor/16x16/actions'
)
