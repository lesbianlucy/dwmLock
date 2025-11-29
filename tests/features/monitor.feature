Feature: Monitor configuration defaults

  Scenario: Default monitor mode is custom
    Given default settings
    Then monitor mode should be custom
    And disable list contains DISPLAY2
