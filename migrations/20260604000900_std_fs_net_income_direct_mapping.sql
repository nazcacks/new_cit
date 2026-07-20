UPDATE public.std_fs_items item
SET is_subtotal = FALSE
FROM public.std_fs_item_versions version
WHERE item.version_id = version.id
  AND version.version_code = 'NTS-2024-GENERAL'
  AND item.stmt_type = 'STD_IS'
  AND item.item_code = '9000';
