#!/bin/bash

mullvad disconnect
sleep 2
mullvad relay set location us
mullvad connect
