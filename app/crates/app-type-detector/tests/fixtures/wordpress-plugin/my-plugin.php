<?php
/*
Plugin Name: My Plugin
Description: Example WordPress plugin.
Version: 0.1.0
Author: Acme
*/

if (!defined('ABSPATH')) exit;

function my_plugin_init() {
    // boot the plugin
}
add_action('plugins_loaded', 'my_plugin_init');
