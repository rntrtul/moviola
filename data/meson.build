subdir('icons')

desktop_file = i18n.merge_file(
        input: 'org.fareedsh.Moviola.desktop.in',
       output: 'org.fareedsh.Moviola.desktop',
         type: 'desktop',
       po_dir: '../po',
      install: true,
  install_dir: datadir / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

appstream_file = i18n.merge_file(
        input: 'org.fareedsh.Moviola.metainfo.xml.in',
       output: 'org.fareedsh.Moviola.metainfo.xml',
       po_dir: '../po',
      install: true,
  install_dir: datadir / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false, disabler: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

install_data('org.fareedsh.Moviola.gschema.xml',
  install_dir: datadir / 'glib-2.0' / 'schemas'
)

compile_schemas = find_program('glib-compile-schemas', required: false, disabler: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_source_dir()])

resources = gnome.compile_resources('resources',
  'resources.gresource.xml',
  gresource_bundle: true,
  source_dir: meson.current_build_dir(),
  install: true,
  install_dir: pkgdatadir,
)
