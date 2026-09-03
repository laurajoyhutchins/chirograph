from typing import Any

from pydantic import BaseModel, computed_field, field_validator


class ValidatorModel(BaseModel):
    value: str

    @field_validator("value", mode="before", json_schema_input_type=int | str)
    @classmethod
    def cast_ints(cls, value: Any) -> Any:
        if isinstance(value, int):
            return str(value)
        return value


class Rectangle(BaseModel):
    width: int
    height: int

    @computed_field
    @property
    def area(self) -> int:
        return self.width * self.height
