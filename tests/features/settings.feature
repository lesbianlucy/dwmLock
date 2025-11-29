Feature: Settings persistence

  Scenario: Toggle blur background
    Given default settings
    When the user disables blur
    Then blur should be disabled

  Scenario: Show monitor text toggle
    Given default settings
    When the user enables monitor text
    Then monitor text should be enabled
